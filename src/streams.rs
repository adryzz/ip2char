use bytes::Bytes;
use packet::ip::v4::Packet;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::{broadcast, mpsc};
use tracing::{info, trace, warn};

use crate::config::Peer;
use crate::types::{Header, MARKER_SIZE, SYNC_MARKER};
use crate::{compression, utils, HEADER_SIZE};

async fn read_from_stream<R>(
    mut stream: ReadHalf<R>,
    mpsc_tx: mpsc::Sender<Bytes>,
    peer: Peer,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0u8; 1600];
    let mut header_buf = [0u8; HEADER_SIZE];
    let mut header: Option<Header> = None;
    let mut desynced = false;

    loop {
        if desynced {
            // packet is malformed, we need to resync to the next packet with marker
            let mut skip: usize = 0;
            loop {
                header_buf.copy_within(1.., 0);
                header_buf[header_buf.len() - 1] = stream.read_u8().await?;
                skip += 1;

                if header_buf[..MARKER_SIZE] == SYNC_MARKER {
                    // found it
                    // read header
                    match Header::from_slice(&header_buf) {
                        Ok(e) => {
                            header = Some(e);
                            desynced = false;
                            info!("[{}] Fixed desync, skipped {} bytes.", peer.path(), skip);
                            break;
                        }
                        Err(e) => {
                            warn!("[{}] Found bad marker: {}", peer.path(), e);
                        }
                    }
                }
            }
        }
        if let Some(h) = header {
            if h.packet_length > 1500 {
                warn!("[{}] Stream desync", peer.path());
                desynced = true;
                continue;
            }
            let b = &mut buf[0..h.packet_length as usize];
            stream.read_exact(b).await?;
            header = None;
            mpsc_tx.send(compression::decompress_into_bytes(b, h.compression).await?).await?;
        } else {
            stream.read_exact(&mut header_buf).await?;
            match Header::from_slice(&header_buf) {
                Ok(e) => header = Some(e),
                Err(e) => {
                    warn!("[{}] Stream desync: {}", peer.path(), e);
                    desynced = true;
                }
            }
        }
    }
}

async fn write_to_stream<W>(
    mut stream: WriteHalf<W>,
    mut broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    peer: Peer,
) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut buf = [0u8; 1600];

    loop {
        let packet: Packet<Bytes>;
        loop {
            match broadcast_rx.recv().await {
                Ok(p) => {
                    packet = p;
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("[{}] Lost {} packets!", peer.path(), n);
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
        }
        // check if packet is for us
        if utils::check_peer_allowed_ip(&packet.destination(), &peer) {
            trace!("Sending packet from kernel");
            // generate a header
            let mut a = Header::default();

            let compressed_size =
                compression::compress_into_buf(packet.as_ref(), &mut buf, peer.compression())
                    .await?;

            a.packet_length = compressed_size as u16;
            a.compression = peer.compression();
            let header_buf: [u8; HEADER_SIZE] = a.into();
            stream.write_all(&header_buf).await?;
            stream.write_all(&buf[..compressed_size]).await?;
        }
    }
}

pub async fn handle_stream<S>(
    stream: S,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mpsc_tx: mpsc::Sender<Bytes>,
    peer: Peer,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Buffer streams for better throughput.
    // Additionally, when recovering from a stream desync,
    // having a buffer helps reduce syscalls when seeking.
    // EDIT: it's currently breaking everything so i disabled it for now
    //let buf_stream = tokio::io::BufStream::new(stream);
    let (read, write) = tokio::io::split(stream);
    let read_task = tokio::task::spawn(read_from_stream(read, mpsc_tx, peer.clone()));
    write_to_stream(write, broadcast_rx, peer).await?;

    read_task.await?
}

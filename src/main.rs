mod config;
mod types;
mod utils;
use crate::config::Config;
use bytes::Bytes;
use config::{CharPeerSection, Peer, SockListenPeerSection, SockPeerSection};
use futures::{SinkExt, StreamExt};
use packet::ip::{self, v4::Packet};
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf},
    select,
    sync::{broadcast, mpsc},
};
use tokio_serial::SerialPortBuilderExt;
use tracing::{error, info, trace, warn};
use tun::TunPacket;
use types::{CompressionType, Header};

const HEADER_SIZE: usize = std::mem::size_of::<Header>();
const MTU: usize = 1500;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    match run().await {
        Ok(_) => info!("ip2char exited successfully."),
        Err(e) => error!("{}", e),
    }
}

async fn run() -> anyhow::Result<()> {
    let config_text = tokio::fs::read_to_string("ip2char.toml").await?;
    let config = toml::from_str::<Config>(&config_text)?;
    info!("[0] Read config file.");

    let mut tun_config = tun::Configuration::default();
    tun_config
        .address(config.interface.address.ip())
        .netmask(config.interface.address.mask())
        .name(&config.interface.name)
        .layer(tun::Layer::L3)
        .mtu(MTU as i32)
        .up();

    #[cfg(target_os = "linux")]
    tun_config.platform(|config| {
        config.packet_information(true);
    });

    let dev = tun::create_as_async(&tun_config)?;
    let mut framed = dev.into_framed();
    info!("[1] Created tun interface.");

    let all_peers = Arc::new(config.get_all_peers());

    if all_peers.len() == 0 {
        warn!("Zero peers listed in configuration file!");
    }

    let (mpsc_tx, mut mpsc_rx) = mpsc::channel(config.interface.buffer.unwrap_or(64));
    let (broadcast_tx, broadcast_rx) = broadcast::channel(config.interface.buffer.unwrap_or(64));

    for peer in all_peers.iter() {
        tokio::task::spawn(connect_to_peer(
            peer.clone(),
            broadcast_rx.resubscribe(),
            mpsc_tx.clone(),
        ));
    }

    loop {
        select! {
            Some(pkt) = framed.next() => handle_packet_from_kernel(pkt?.into_bytes(), &broadcast_tx).await?,
            Some(data) = mpsc_rx.recv() => {
                let packet = prep_packet_for_kernel(data).await?;
                framed.send(packet).await?;
            }
        };
    }
}

async fn handle_packet_from_kernel(
    data: Bytes,
    tx: &broadcast::Sender<Packet<Bytes>>,
) -> anyhow::Result<()> {
    match ip::Packet::new(data) {
        Ok(ip::Packet::V4(pkt)) => {
            tx.send(pkt)?;
        }
        Ok(ip::Packet::V6(_pkt)) => {
            //tracing::trace!("V6 packet, cant do anything about it for now");
        }
        Err(err) => println!("Received an invalid packet: {:?}", err),
    }

    Ok(())
}

async fn prep_packet_for_kernel(packet: Bytes) -> anyhow::Result<TunPacket> {
    trace!("Sending packet to kernel");

    // ugh very bad for performance
    Ok(TunPacket::new(packet.to_vec()))
}

async fn connect_to_peer(
    peer: Peer,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mspc_tx: mpsc::Sender<Bytes>,
) {
    info!("Connecting to {}...", peer.path());
    let path = peer.path().to_string();
    let res = match peer {
        Peer::Char(c) => connect_serial(c, broadcast_rx, mspc_tx).await,
        Peer::Sock(s) => connect_sock(s, broadcast_rx, mspc_tx).await,
        Peer::SockListen(s) => connect_sock_listen(s, broadcast_rx, mspc_tx).await,
    };

    match res {
        Ok(_) => info!("Connection to {} closed successfully.", path),
        Err(e) => error!("[{}] Error: {}", path, e),
    }
}

async fn connect_sock(
    peer: SockPeerSection,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mpsc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let stream = tokio::net::TcpStream::connect(&peer.path).await?;
    info!("Connected to {}.", &peer.path);

    handle_stream(stream, broadcast_rx, mpsc_tx, Peer::Sock(peer)).await?;

    Ok(())
}

async fn connect_sock_listen(
    peer: SockListenPeerSection,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mpsc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(&peer.path).await?;
    let (stream, _) = listener.accept().await?;
    info!("Connected to {}.", &peer.path);

    handle_stream(stream, broadcast_rx, mpsc_tx, Peer::SockListen(peer)).await?;

    Ok(())
}

async fn read_from_stream<R>(
    mut stream: ReadHalf<R>,
    mpsc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0u8; 1500];
    let mut header_buf = [0u8; HEADER_SIZE];
    let mut header: Option<Header> = None;

    loop {
        if let Some(h) = header {
            let b = &mut buf[0..h.packet_length as usize];
            stream.read_exact(b).await?;
            header = None;
            mpsc_tx.send(Bytes::copy_from_slice(b)).await?;
        } else {
            stream.read_exact(&mut header_buf).await?;
            header = Some(Header::from_slice(&header_buf)?);
        }
    }
}

async fn write_to_stream<W>(
    mut stream: WriteHalf<W>,
    mut broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    peer: &Peer,
) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
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
        if utils::check_peer_allowed_ip(&packet.destination(), peer) {
            trace!("Sending packet from kernel");
            // generate a header
            let mut a = Header::default();
            a.packet_length = packet.length();
            let header_buf: [u8; HEADER_SIZE] = a.into();
            stream.write_all(&header_buf).await?;
            stream.write_all(packet.as_ref()).await?;
        }
    }
}

async fn handle_stream<S>(
    stream: S,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mpsc_tx: mpsc::Sender<Bytes>,
    peer: Peer,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (read, write) = tokio::io::split(stream);
    let read_task = tokio::task::spawn(read_from_stream(read, mpsc_tx));
    write_to_stream(write, broadcast_rx, &peer).await?;

    read_task.await?
}

async fn connect_serial(
    peer: CharPeerSection,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mspc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let port = tokio_serial::new(&peer.path, peer.speed.unwrap_or(115200)).open_native_async()?;
    info!("Connected to {}.", &peer.path);

    handle_stream(port, broadcast_rx, mspc_tx, Peer::Char(peer)).await?;
    Ok(())
}

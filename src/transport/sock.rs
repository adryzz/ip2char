use bytes::Bytes;
use packet::ip::v4::Packet;
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use crate::config::{Peer, SockListenPeerSection, SockPeerSection};
use crate::streams::handle_stream;

pub async fn connect_sock(
    peer: SockPeerSection,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mpsc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let stream = tokio::net::TcpStream::connect(&peer.path).await?;
    info!("Connected to {}.", &peer.path);

    handle_stream(stream, broadcast_rx, mpsc_tx, Peer::Sock(peer)).await?;

    Ok(())
}

pub async fn connect_sock_listen(
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

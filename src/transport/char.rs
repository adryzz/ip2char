use bytes::Bytes;
use packet::ip::v4::Packet;
use tokio::sync::{broadcast, mpsc};
use tokio_serial::SerialPortBuilderExt;
use tracing::info;

use crate::config::{CharPeerSection, Peer};
use crate::streams::handle_stream;

pub async fn connect_serial(
    peer: CharPeerSection,
    broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mspc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let port = tokio_serial::new(&peer.path, peer.speed.unwrap_or(115200)).open_native_async()?;
    info!("Connected to {}.", &peer.path);

    handle_stream(port, broadcast_rx, mspc_tx, Peer::Char(peer)).await?;
    Ok(())
}

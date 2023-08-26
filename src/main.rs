mod config;
mod streams;
mod transport;
mod tun_device;
mod types;
mod utils;
mod packet_handling;

use crate::config::parse_config;
use crate::packet_handling::{handle_packet_from_kernel, prep_packet_for_kernel};
use crate::transport::char::connect_serial;
use crate::transport::sock::{connect_sock, connect_sock_listen};
use bytes::Bytes;
use config::Peer;
use futures::{SinkExt, StreamExt};
use packet::ip::v4::Packet;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};
use tun_device::create_tun;
use types::Header;

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
    let (config, all_peers) = parse_config().await?;
    let mut framed = create_tun(&config)?;

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
            Some(pkt) = framed.next() => handle_packet_from_kernel(pkt?.into_bytes(), &broadcast_tx)?,
            Some(data) = mpsc_rx.recv() => {
                let packet = prep_packet_for_kernel(data)?;
                framed.send(packet).await?;
            }
        };
    }
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
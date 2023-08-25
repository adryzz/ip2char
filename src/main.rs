mod config;
mod types;
mod utils;
use crate::config::Config;
use bytes::Bytes;
use config::{CharPeerSection, Peer, SockListenPeerSection, SockPeerSection};
use futures::{SinkExt, StreamExt};
use packet::{
    ip::{self, v4::Packet},
    AsPacket,
};
use std::sync::Arc;
use tokio::{
    select,
    sync::{broadcast, mpsc}, io::{AsyncReadExt, Interest},
};
use tokio_serial::SerialPortBuilderExt;
use tracing::{error, info, warn};
use tun::TunPacket;
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
    let config_text = tokio::fs::read_to_string("ip2char.toml").await?;
    let config = toml::from_str::<Config>(&config_text)?;
    info!("[0] Read config file.");

    let mut tun_config = tun::Configuration::default();
    tun_config
        .address(config.interface.address.first_as_ipv4_addr())
        .netmask(config.interface.address.get_mask_as_ipv4_addr())
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

    let (mpsc_tx, mut mpsc_rx) = mpsc::channel(64);
    let (broadcast_tx, broadcast_rx) = broadcast::channel(64);

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
        Ok(ip::Packet::V6(pkt)) => {
            //tracing::trace!("V6 packet, cant do anything about it for now");
        }
        Err(err) => println!("Received an invalid packet: {:?}", err),
    }

    Ok(())
}

async fn prep_packet_for_kernel(packet: Bytes) -> anyhow::Result<TunPacket> {
    info!("prepping packet");
    let hex_string = packet[..].iter().map(|&x| format!("{:02X}", x)).collect::<Vec<String>>().join(" ");
    info!("{}", hex_string);

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
    mut broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mut mspc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    info!("Connected to {}.", &peer.path);
    Ok(())
}

async fn connect_sock_listen(
    peer: SockListenPeerSection,
    mut broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mut mspc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(&peer.path).await?;
    let (mut stream, _) = listener.accept().await?;
    info!("Connected to {}.", &peer.path);

    let mut buf = [0u8; 1500];
    let mut header_buf = [0u8; HEADER_SIZE];
    let mut header: Option<Header> = None;

    loop {
        let ready = stream.ready(Interest::READABLE).await?;
        if ready.is_readable() {
            if let Some(h) = header {
                let b = &mut buf[0..h.packet_length as usize];
                stream.read_exact(b).await?;
                header = None;
                mspc_tx.send(Bytes::copy_from_slice(b)).await?;
            } else {
                stream.read_exact(&mut header_buf).await?;
                header = Some(Header::from_slice(&header_buf)?);
            }
        }
    }
}

async fn connect_serial(
    peer: CharPeerSection,
    mut broadcast_rx: broadcast::Receiver<Packet<Bytes>>,
    mut mspc_tx: mpsc::Sender<Bytes>,
) -> anyhow::Result<()> {
    let mut port =
        tokio_serial::new(&peer.path, peer.speed.unwrap_or(115200)).open_native_async()?;
    info!("Connected to {}.", &peer.path);

    while let Ok(packet) = broadcast_rx.recv().await {}
    Ok(())
}

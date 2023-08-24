mod config;
mod types;
mod utils;


use config::{Peer, CharPeerSection};
use futures::{SinkExt, StreamExt};
use packet::{builder::Builder, ip};

use tokio::select;
use tun::{TunPacket};
use std::{sync::{Arc}};
use tracing::{error, info};
use types::Header;

use crate::config::Config;

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
        .mtu(1492 - std::mem::size_of::<Header>() as i32)
        .up();

    #[cfg(target_os = "linux")]
    tun_config.platform(|config| {
        config.packet_information(true);
    });

    let dev = tun::create_as_async(&tun_config)?;
    let mut framed = dev.into_framed();
    info!("[1] Created tun interface.");

    let all_peers = Arc::new(config.get_all_peers());

    for peer in all_peers.iter() {
        tokio::task::spawn(connect_to_peer(peer.clone()));
    }

    loop {
        select! {
            Some(pkt) = framed.next() => handle_packet_from_kernel(pkt?).await,
            Some(data) = async { Some(&[
                0x45,0x00,0x00,0x54,0x6d,0xfd,0x40,0x00,0x40,0x01,0xb8,0xa8,0x0a,0x01,0x00,0x00,0x0a,0x01,0x00,0x02,0x08,0x00,0x39,0xbe,0x00,0x10,0x00,0x01,0x82,0xbe,0xe7,0x64,0x00,0x00,0x00,0x00,0x93,0x3a,0x02,0x00,0x00,0x00,0x00,0x00,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x18,0x19,0x1a,0x1b,0x1c,0x1d,0x1e,0x1f,0x20,0x21,0x22,0x23,0x24,0x25,0x26,0x27,0x28,0x29,0x2a,0x2b,0x2c,0x2d,0x2e,0x2f,0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37
                ]) } => {
                framed.send(prep_packet_for_kernel(data).await?).await?;
            }
        };
    }

    Ok(())
}

async fn handle_packet_from_kernel(pkt: TunPacket) {
    info!("handling packet from kernel");
    match ip::Packet::new(pkt.get_bytes()) {
        Ok(ip::Packet::V4(_pkt)) => {
            // for peer in all_peers.iter() {
            //     if utils::check_peer_allowed_ip(&pkt.destination(), peer) {
            //         //tracing::info!("V4 packet to {}, to be routed to {}", pkt.destination(), peer.path());
            //     } else {
            //         //tracing::trace!("V4 packet to {}, NOT routed to {}", pkt.destination(), peer.path());
            //     }
            // }
        }
        Ok(ip::Packet::V6(_pkt)) => {
            //tracing::trace!("V6 packet, cant do anything about it for now");
        }
        Err(err) => println!("Received an invalid packet: {:?}", err),
    }
}

async fn prep_packet_for_kernel(data: &[u8]) -> anyhow::Result<TunPacket> {
    info!("prepping packet");
    // todo: actually make it decode the header and stuff lol
    Ok(TunPacket::new(data.to_vec()))
}

async fn connect_to_peer(peer: Peer) -> anyhow::Result<()> {
    info!("Connecting to {}...", peer.path());
    match peer {
        Peer::Char(c) => connect_serial(c).await,
    }
}

async fn connect_serial(_peer: CharPeerSection) -> anyhow::Result<()> {
    Ok(())
}
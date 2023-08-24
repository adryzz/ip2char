mod config;
mod types;
mod utils;

use cidr_utils::cidr::IpCidr;
use futures::{SinkExt, StreamExt};
use packet::{builder::Builder, icmp, ip, Packet};
use pdu::{Ethernet, EthernetPdu, Ip};
use std::{io::Read, sync::Arc};
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

    while let Some(pkt) = framed.next().await {
        let pkt = pkt?;
        match ip::Packet::new(pkt.get_bytes()) {
            Ok(ip::Packet::V4(pkt)) => {
                for peer in all_peers.iter() {
                    if utils::check_peer_allowed_ip(&pkt.destination(), peer) {
                        //tracing::info!("V4 packet to {}, to be routed to {}", pkt.destination(), peer.path());
                    } else {
                        //tracing::trace!("V4 packet to {}, NOT routed to {}", pkt.destination(), peer.path());
                    }
                }
            }
            Ok(ip::Packet::V6(pkt)) => {
                //tracing::trace!("V6 packet, cant do anything about it for now");
            }
            Err(err) => println!("Received an invalid packet: {:?}", err),
            _ => {}
        }
    }

    Ok(())
}

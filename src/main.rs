mod config;
mod types;

use std::io::Read;
use packet::{builder::Builder, icmp, ip, Packet};
use cidr_utils::cidr::IpCidr;
use pdu::{Ip, EthernetPdu, Ethernet};
use futures::{SinkExt, StreamExt};
use tracing::{info, error};
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
    let config_text = tokio::fs::read_to_string("ip2char0.toml").await?;
    let config = toml::from_str::<Config>(&config_text)?;

    let mut tun_config = tun::Configuration::default();
    tun_config
        .address(config.interface.address.first_as_ipv4_addr())
        .netmask(config.interface.address.get_mask_as_ipv4_addr())
        .name("ip2char0")
        .layer(tun::Layer::L3)
        .mtu(1492 - std::mem::size_of::<Header>() as i32)
        .up();

    #[cfg(target_os = "linux")]
    tun_config.platform(|config| {
        config.packet_information(true);
    });

    let dev = tun::create_as_async(&tun_config).unwrap();
    let mut framed = dev.into_framed();

    while let Some(packet) = framed.next().await {
        match packet {
            Ok(pkt) => match ip::Packet::new(pkt.get_bytes()) {
                Ok(ip::Packet::V4(pkt)) => {
                    for peer in config.peer_char.iter() {
                        let mut allowed = false;
                        for range in peer.allowedips.iter() {
                            if range.contains(pkt.destination()) {
                                allowed = true;
                            }
                        }

                        if allowed {
                            tracing::info!("V4 packet to {}, to be routed to {}", pkt.destination(), peer.path);
                        } else {
                            tracing::trace!("V4 packet to {}, NOT routed to {}", pkt.destination(), peer.path);
                        }
                    }
                },
                Ok(ip::Packet::V6(pkt)) => {
                    tracing::trace!("V6 packet, cant do anything about it for now");
                }
                Err(err) => println!("Received an invalid packet: {:?}", err),
                _ => {}
            },
            Err(err) => panic!("Error: {:?}", err),
        }
    }
    Ok(())
}
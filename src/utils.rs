use ipnetwork::IpNetwork;
use tokio::process::Command;
use tokio::signal;
use tracing::{error, info};

use crate::config::Peer;
use std::net::Ipv4Addr;
use std::process::exit;

pub fn check_peer_allowed_ip(ip: &Ipv4Addr, peer: &Peer) -> bool {
    let mut allowed = false;
    for range in peer.allowed_ips().iter() {
        // TODO: investigate IPv6 support
        if let IpNetwork::V4(a) = range {
            if a.contains(*ip) {
                allowed = true;
            }
        }
    }

    allowed
}

pub fn run_command(command_str: &str) -> anyhow::Result<()> {
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(command_str).spawn()?;

    Ok(())
}

pub async fn handle_post_down_command_sigint(post_down: String) -> anyhow::Result<()> {
    signal::ctrl_c().await?;
    match run_command(&post_down) {
        Ok(_) => info!("post-down: {}", &post_down),
        Err(e) => error!("post-down: {}", e),
    }
    exit(0);
}

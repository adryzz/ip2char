use ipnetwork::{IpNetwork, Ipv4Network};

use crate::config::Peer;
use std::net::Ipv4Addr;

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

    return allowed;
}

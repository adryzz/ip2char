use cidr_utils::cidr::Ipv4Cidr;

use crate::config::Peer;
use std::net::Ipv4Addr;

pub fn check_peer_allowed_ip(ip: &Ipv4Addr, peer: &Peer) -> bool {
    let mut allowed = false;
    for range in peer.allowed_ips().iter() {
        if range.contains(ip) {
            allowed = true;
        }
    }

    return allowed;
}

pub fn check_allowed_ip(ip: &Ipv4Addr, allowed_ips: &[Ipv4Cidr]) -> bool {
    let mut allowed = false;
    for range in allowed_ips.iter() {
        if range.contains(ip) {
            allowed = true;
        }
    }

    return allowed;
}

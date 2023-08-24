use cidr_utils::cidr::{IpCidr, Ipv4Cidr, Ipv6Cidr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub interface: InterfaceSection,
    #[serde(rename = "peer-char")]
    pub peer_char: Vec<CharPeerSection>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSection {
    pub address: Ipv4Cidr
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharPeerSection {
    pub path: String,
    pub allowedips: Vec<Ipv4Cidr>,
}
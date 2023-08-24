use cidr_utils::cidr::{Ipv4Cidr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub interface: InterfaceSection,
    #[serde(rename = "peer-char")]
    #[serde(default)]
    pub peer_char: Vec<CharPeerSection>,
}

impl Config {
    pub fn get_all_peers(&self) -> Vec<Peer> {
        let mut vec = Vec::new();
        for c in &self.peer_char {
            vec.push(Peer::Char(c.clone()));
        }

        vec
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSection {
    pub address: Ipv4Cidr,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharPeerSection {
    pub path: String,
    pub allowedips: Vec<Ipv4Cidr>,
    pub speed: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum Peer {
    Char(CharPeerSection),
}

impl Peer {
    pub fn allowed_ips(&self) -> &[Ipv4Cidr] {
        match self {
            Peer::Char(c) => &c.allowedips[..],
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Peer::Char(c) => &c.path,
        }
    }
}

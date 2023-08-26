use ipnetwork::Ipv4Network;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub interface: InterfaceSection,

    #[serde(rename = "peer-char")]
    #[serde(default)]
    pub peer_char: Vec<CharPeerSection>,

    #[serde(rename = "peer-sock")]
    #[serde(default)]
    pub peer_sock: Vec<SockPeerSection>,

    #[serde(rename = "peer-sock-listen")]
    #[serde(default)]
    pub peer_sock_listen: Vec<SockListenPeerSection>,
}

impl Config {
    pub fn get_all_peers(&self) -> Vec<Peer> {
        let mut vec = Vec::new();
        for c in &self.peer_char {
            vec.push(Peer::Char(c.clone()));
        }

        for s in &self.peer_sock {
            vec.push(Peer::Sock(s.clone()));
        }

        for s in &self.peer_sock_listen {
            vec.push(Peer::SockListen(s.clone()));
        }

        vec
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSection {
    pub address: Ipv4Network,
    pub name: String,
    #[serde(rename = "ip-filtering")]
    pub ip_filtering: Option<bool>,
    pub buffer: Option<usize>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharPeerSection {
    pub path: String,
    pub allowedips: Vec<Ipv4Network>,
    pub speed: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SockPeerSection {
    pub path: String,
    pub allowedips: Vec<Ipv4Network>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SockListenPeerSection {
    pub path: String,
    pub allowedips: Vec<Ipv4Network>,
}

#[derive(Debug, Clone)]
pub enum Peer {
    Char(CharPeerSection),
    Sock(SockPeerSection),
    SockListen(SockListenPeerSection),
}

impl Peer {
    pub fn allowed_ips(&self) -> &[Ipv4Network] {
        match self {
            Peer::Char(c) => &c.allowedips[..],
            Peer::Sock(c) => &c.allowedips[..],
            Peer::SockListen(c) => &c.allowedips[..],
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Peer::Char(c) => &c.path,
            Peer::Sock(c) => &c.path,
            Peer::SockListen(c) => &c.path,
        }
    }
}

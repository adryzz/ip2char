use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::types::{CompressionType, EncryptionType};

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
    pub address: IpNetwork,
    pub name: String,
    #[serde(rename = "ip-filtering")]
    pub ip_filtering: Option<bool>,
    pub buffer: Option<usize>,
    #[serde(rename = "post-up")]
    pub post_up: Option<String>,
    #[serde(rename = "post-down")]
    pub post_down: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharPeerSection {
    pub path: String,
    pub allowedips: Vec<IpNetwork>,
    pub speed: Option<u32>,
    #[serde(default)]
    pub compression: Option<CompressionType>,
    #[serde(default)]
    pub encryption: Option<EncryptionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SockPeerSection {
    pub path: String,
    pub allowedips: Vec<IpNetwork>,
    #[serde(default)]
    pub compression: Option<CompressionType>,
    #[serde(default)]
    pub encryption: Option<EncryptionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SockListenPeerSection {
    pub path: String,
    pub allowedips: Vec<IpNetwork>,
    #[serde(default)]
    pub compression: Option<CompressionType>,
    #[serde(default)]
    pub encryption: Option<EncryptionType>,
}

#[derive(Debug, Clone)]
pub enum Peer {
    Char(CharPeerSection),
    Sock(SockPeerSection),
    SockListen(SockListenPeerSection),
}

impl Peer {
    pub fn allowed_ips(&self) -> &[IpNetwork] {
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

    pub fn compression(&self) -> CompressionType {
        match self {
            Peer::Char(c) => c.compression.unwrap_or(CompressionType::None),
            Peer::Sock(c) => c.compression.unwrap_or(CompressionType::None),
            Peer::SockListen(c) => c.compression.unwrap_or(CompressionType::None),
        }
    }
}

pub async fn parse_config() -> anyhow::Result<(Config, Vec<Peer>)> {
    let config_text = tokio::fs::read_to_string("ip2char.toml").await?;
    let config = toml::from_str::<Config>(&config_text)?;
    info!("[0] Read config file.");

    let all_peers = config.get_all_peers();

    if all_peers.is_empty() {
        warn!("Zero peers listed in configuration file!");
    }

    Ok((config, all_peers))
}

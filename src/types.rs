use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

const VERSION: u16 = 0;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Header {
    pub version: u16,
    pub packet_length: u16,
    pub compression: CompressionType,
    pub encryption: EncryptionType,
    _reserved: [u8; 10],
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: VERSION,
            ..Default::default()
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
pub enum CompressionType {
    #[default]
    None,
}

#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
pub enum EncryptionType {
    #[default]
    None,
}

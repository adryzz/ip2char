use serde::{Deserialize, Serialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

const VERSION: u16 = 0;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Header {
    version: u16,
    compression: CompressionType,
    encryption: EncryptionType,
    _reserved: [u8; 12]
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
    None
}

#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
pub enum EncryptionType {
    #[default]
    None
}
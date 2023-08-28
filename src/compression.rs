use std::io::Cursor;

use anyhow::Ok;
use async_compression::tokio::bufread::{GzipDecoder, ZstdDecoder};
use async_compression::tokio::write::{GzipEncoder, ZstdEncoder};
use async_compression::Level;
use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::types::CompressionType;

pub async fn compress_into_buf(
    data: &[u8],
    buf: &mut [u8],
    compression_type: CompressionType,
) -> anyhow::Result<usize> {
    let mut cursor = Cursor::new(buf);
    match compression_type {
        CompressionType::None => {
            cursor.write_all(data).await?;
            Ok(data.len())
        }
        CompressionType::Zstd => {
            let mut encoder = ZstdEncoder::with_quality(&mut cursor, Level::Default);
            encoder.write_all(data).await?;
            encoder.flush().await?;
            encoder.shutdown().await?;

            Ok(cursor.position() as usize + 1)
        }
        CompressionType::ZstdFast => {
            let mut encoder = ZstdEncoder::with_quality(&mut cursor, Level::Fastest);
            encoder.write_all(data).await?;
            encoder.flush().await?;
            encoder.shutdown().await?;

            Ok(cursor.position() as usize + 1)
        }
        CompressionType::ZstdSlow => {
            let mut encoder = ZstdEncoder::with_quality(&mut cursor, Level::Best);
            encoder.write_all(data).await?;
            encoder.flush().await?;
            encoder.shutdown().await?;

            Ok(cursor.position() as usize + 1)
        }
        CompressionType::Gzip => {
            let mut encoder = GzipEncoder::with_quality(&mut cursor, Level::Default);
            encoder.write_all(data).await?;
            encoder.flush().await?;
            encoder.shutdown().await?;

            Ok(cursor.position() as usize + 1)
        }
    }
}

pub async fn decompress_into_bytes(
    data: &[u8],
    compression_type: CompressionType,
) -> anyhow::Result<Bytes> {
    let mut buf = Vec::with_capacity(data.len());
    match compression_type {
        CompressionType::None => Ok(Bytes::copy_from_slice(data)),
        CompressionType::Zstd | CompressionType::ZstdFast | CompressionType::ZstdSlow => {
            let mut decoder = ZstdDecoder::new(data);
            decoder.read_to_end(&mut buf).await?;

            Ok(buf.into())
        }
        CompressionType::Gzip => {
            let mut decoder = GzipDecoder::new(data);
            decoder.read_to_end(&mut buf).await?;

            Ok(buf.into())
        }
    }
}

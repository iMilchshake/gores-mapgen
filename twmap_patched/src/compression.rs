use flate2::write::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use std::cmp::Ordering;
use std::io;
use std::io::Write;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZlibDecompressionError {
    #[error("Io: {0}")]
    Io(#[from] io::Error),
    #[error("Decompressed data is larger than advertised")]
    TooBig,
    #[error("Decompressed data is smaller than advertised")]
    TooSmall,
}

pub fn decompress(data: &[u8], expected_size: usize) -> Result<Vec<u8>, ZlibDecompressionError> {
    let mut decoder = ZlibDecoder::new(Vec::new());
    decoder.write_all(data)?;
    let decompressed = decoder.finish()?;
    match decompressed.len().cmp(&expected_size) {
        Ordering::Less => Err(ZlibDecompressionError::TooSmall),
        Ordering::Greater => Err(ZlibDecompressionError::TooBig),
        Ordering::Equal => Ok(decompressed),
    }
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(9));
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

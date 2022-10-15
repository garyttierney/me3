use std::io::{Read, Write};

mod read;
mod write;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

pub use self::read::DcxDecoder;
pub use self::write::{DcxBuilder, DcxEncoder, DcxWriter};

#[derive(Debug, PartialEq, Eq)]
pub struct DcxHeader {
    pub(crate) version: u32,
    pub(crate) compression_parameters: CompressionParameters,
    pub(crate) compressed_sizes: CompressedSizes,
    pub(crate) chunk_info: ChunkInfo,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChunkInfo {}

#[derive(Debug, PartialEq, Eq)]
pub struct CompressedSizes {
    pub(crate) uncompressed_size: u32,
    pub(crate) compressed_size: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CompressionParameters {
    Deflate { compression_level: u8 },
    Kraken {},
    Edge,
}

impl CompressionParameters {
    pub fn deflate_fast() -> Self {
        CompressionParameters::Deflate {
            compression_level: 9,
        }
    }

    pub fn create_decoder<R: Read>(&self, inner: R) -> DcxDecoder<R> {
        match self {
            CompressionParameters::Deflate { .. } => DcxDecoder::Deflate(ZlibDecoder::new(inner)),
            _ => unimplemented!("{:#?} decompression is not yet supported", &self),
        }
    }

    pub fn create_encoder<W: Write>(&self, inner: W) -> DcxEncoder<W> {
        match self {
            CompressionParameters::Deflate { compression_level } => DcxEncoder::Deflate(
                ZlibEncoder::new(inner, Compression::new(*compression_level as u32)),
            ),
            _ => unimplemented!("{:#?} compression is not yet supported", &self),
        }
    }

    pub fn id(&self) -> &'static [u8] {
        match self {
            Self::Deflate { .. } => b"DFLT",
            Self::Kraken {} => b"KRAK",
            Self::Edge => b"EDGE",
        }
    }
}

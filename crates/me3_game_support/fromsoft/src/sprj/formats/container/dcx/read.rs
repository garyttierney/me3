use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;

use super::{ChunkInfo, CompressedSizes, CompressionParameters, DcxHeader};
use crate::sprj::formats::read::ReadFormatsExt;

pub enum DcxDecoder<R: Read> {
    Deflate(ZlibDecoder<R>),
}

pub fn read_compression_params<R: Read>(
    mut reader: R,
) -> Result<CompressionParameters, std::io::Error> {
    let mut algo = [0u8; 4];
    reader.read_exact(&mut algo)?;

    let algo_settings_len = reader.read_u32::<BigEndian>()?;
    let mut algo_settings = Vec::with_capacity(algo_settings_len as usize);
    reader.read_to_end(&mut algo_settings)?;

    let compression_level = algo_settings[0];

    match &algo {
        b"DFLT" => Ok(CompressionParameters::Deflate { compression_level }),
        _ => unimplemented!(),
    }
}

impl<R: Read> DcxDecoder<R> {
    pub fn new(mut reader: R) -> Result<(DcxHeader, DcxDecoder<R>), std::io::Error> {
        reader.read_magic(b"DCX\0")?;

        let version = reader.read_u32::<BigEndian>()?;
        let _size_offset = reader.read_u32::<BigEndian>()?;
        let _params_offset = reader.read_u32::<BigEndian>()?;
        let _chunk_info_offset = reader.read_u32::<BigEndian>()?;
        let _data_offset = reader.read_u32::<BigEndian>()?;

        // A "compliant" DCX parser would seek to the offsets specified for each block. For now
        // this parser assumes the blocks are tightly packed and laid out in sequence.

        reader.read_magic(b"DCS\0")?;
        let uncompressed_size = reader.read_u32::<BigEndian>()?;
        let compressed_size = reader.read_u32::<BigEndian>()?;
        let compressed_sizes = CompressedSizes {
            uncompressed_size,
            compressed_size,
        };

        reader.read_magic(b"DCP\0")?;
        let compression_parameters = read_compression_params(&mut reader)?;

        reader.read_magic(b"DCA\0")?;
        let _chunk_info_len = reader.read_u32::<BigEndian>()?;
        let chunk_info = ChunkInfo {};

        let decoder = compression_parameters.create_decoder(reader);
        let header = DcxHeader {
            version,
            compression_parameters,
            compressed_sizes,
            chunk_info,
        };

        Ok((header, decoder))
    }
}

impl<R: Read> Read for DcxDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            DcxDecoder::Deflate(compressor) => compressor.read(buf),
        }
    }
}

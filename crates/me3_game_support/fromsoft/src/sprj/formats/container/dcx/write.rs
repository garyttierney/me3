use std::io::{Seek, Write};

use byteorder::{BigEndian, WriteBytesExt};
use flate2::write::ZlibEncoder;

use super::CompressionParameters;
use crate::sprj::formats::write::{Unresolved, WriteFormatsExt};

/// A builder structure to create a new [DcxWriter].
///
/// This structure controls header configuration options such as the format version and compression
/// parameters.
pub struct DcxBuilder {
    version: u32,
}

impl DcxBuilder {
    pub fn new(version: u32) -> Self {
        Self { version }
    }

    /// Consume this builder by writing the header out to [writer] and creating a [DcxWriter].
    ///
    /// Data written to the returned writer will be compressed and then written to the supplied
    /// [writer].
    pub fn write<W: Write + Seek>(
        self,
        mut writer: W,
        compression_params: CompressionParameters,
    ) -> std::io::Result<DcxWriter<W>> {
        writer.write_all(b"DCX\0")?;
        writer.write_u32::<BigEndian>(self.version)?;

        let mut sizes_offset = writer.write_unresolved_u32::<BigEndian>()?;
        let mut params_offset = writer.write_unresolved_u32::<BigEndian>()?;
        let mut chunk_info_offset = writer.write_unresolved_u32::<BigEndian>()?;
        let mut data_offset = writer.write_unresolved_u32::<BigEndian>()?;

        // Write the data size section
        let _ = sizes_offset.resolve_with_position(&mut writer)?;
        writer.write_all(b"DCS\0")?;
        let uncompressed_size = writer.write_unresolved_u32::<BigEndian>()?;
        let compressed_size = writer.write_unresolved_u32::<BigEndian>()?;

        // Write the compression parameters section
        let params_offset = params_offset.resolve_with_position(&mut writer)?;
        writer.write_all(b"DCP\0")?;
        writer.write_all(compression_params.id())?;

        let mut compression_params_len = writer.write_unresolved_u32::<BigEndian>()?;
        match compression_params {
            CompressionParameters::Deflate { compression_level } => {
                writer.write_all(&[compression_level, 0, 0, 0])?;
                writer.write_u32::<BigEndian>(0)?;
                writer.write_u32::<BigEndian>(0)?;
                writer.write_u32::<BigEndian>(0)?;
                writer.write_u32::<BigEndian>(0x00010100)?;
            }
            _ => unimplemented!(),
        }
        compression_params_len.resolve_with_relative_offset(&mut writer, params_offset as u64)?;

        // Write the compression chunk info section
        let chunk_info_offset = chunk_info_offset.resolve_with_position(&mut writer)?;
        writer.write_all(b"DCA\0")?;
        let mut chunk_info_len = writer.write_unresolved_u32::<BigEndian>()?;
        chunk_info_len.resolve_with_relative_offset(&mut writer, chunk_info_offset as u64)?;

        // Mark the beginning of the data section and hand the writer over to the encoder
        data_offset.resolve_with_position(&mut writer)?;
        let encoder = compression_params.create_encoder(writer);

        Ok(DcxWriter {
            encoder,
            uncompressed_size,
            compressed_size,
        })
    }
}

pub struct DcxWriter<W: Write + Seek> {
    encoder: DcxEncoder<W>,
    compressed_size: Unresolved<u32, BigEndian>,
    uncompressed_size: Unresolved<u32, BigEndian>,
}

pub enum DcxEncoder<W: Write> {
    Deflate(ZlibEncoder<W>),
}

impl<W: Write> DcxEncoder<W> {
    pub fn bytes_out(&self) -> usize {
        match self {
            Self::Deflate(zlib) => zlib.total_out() as usize,
        }
    }

    pub fn bytes_in(&self) -> usize {
        match self {
            Self::Deflate(zlib) => zlib.total_in() as usize,
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match self {
            Self::Deflate(zlib) => zlib.write(buf),
        }
    }

    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        match self {
            Self::Deflate(zlib) => zlib.try_finish(),
        }
    }

    pub fn finish(self) -> Result<W, std::io::Error> {
        match self {
            Self::Deflate(zlib) => zlib.finish(),
        }
    }
}

impl<W: Write + Seek> Write for DcxWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.encoder.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.encoder.flush()
    }
}

impl<W: Write + Seek> DcxWriter<W> {
    pub fn finish(mut self) -> Result<W, std::io::Error> {
        self.encoder.flush()?;

        let total_out = self.encoder.bytes_out();
        let total_in = self.encoder.bytes_in();
        let mut writer = self.encoder.finish()?;

        self.uncompressed_size
            .resolve(&mut writer, total_in as u32)?;
        self.compressed_size
            .resolve(&mut writer, total_out as u32)?;

        Ok(writer)
    }
}

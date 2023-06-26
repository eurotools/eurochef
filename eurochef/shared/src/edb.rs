use std::io::{Read, Seek};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    versions::Platform,
};
use tracing::info;

use crate::error::Result;

pub trait DatabaseReader: Read + Seek {}

impl<R: Read + Seek + Sized> DatabaseReader for R {}

pub struct EdbFile<'d> {
    reader: &'d mut dyn DatabaseReader,
    pub endian: Endian,
    pub platform: Platform,
    pub header: EXGeoHeader,
}

impl<'d> EdbFile<'d> {
    /// Resets the reader, tests endianness and reads the header
    pub fn new(reader: &'d mut dyn DatabaseReader, platform: Platform) -> Result<Self> {
        let mut reader = reader;
        reader.seek(std::io::SeekFrom::Start(0)).ok();
        let endian = if reader.read_ne::<u8>()? == 0x47 {
            Endian::Big
        } else {
            Endian::Little
        };

        reader.seek(std::io::SeekFrom::Start(8))?;
        let version = reader.read_type::<u32>(endian)?;
        if version >= 0x10000 {
            return Err(crate::error::EurochefError::Unsupported(
                crate::error::UnsupportedError::EngineXT(version),
            ));
        }

        if version < 182 || version > 263 {
            return Err(crate::error::EurochefError::Unsupported(
                crate::error::UnsupportedError::Version(version),
            ));
        }

        reader.seek(std::io::SeekFrom::Start(0))?;

        let header = reader.read_type::<EXGeoHeader>(endian)?;

        info!(
            "Loaded EDB {:08x} v{} (build date {}, size 0x{:x}, platform {})",
            header.hashcode,
            header.version,
            chrono::NaiveDateTime::from_timestamp_opt(header.time as _, 0).unwrap(),
            header.file_size,
            platform
        );

        Ok(Self {
            reader,
            endian,
            platform,
            header,
        })
    }
}

impl<'d> Seek for EdbFile<'d> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }
}

impl<'d> Read for EdbFile<'d> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

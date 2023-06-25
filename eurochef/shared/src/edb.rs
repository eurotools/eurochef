use std::io::{Read, Seek};

use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    versions::Platform,
};
use tracing::debug;

use crate::error::Result;

pub struct DatabaseFile<T: Read + Seek> {
    reader: T,
    pub endian: Endian,
    pub platform: Platform,
    pub header: EXGeoHeader,
}

impl<T: Read + Seek> DatabaseFile<T> {
    /// Resets the reader, tests endianness and reads the header
    pub fn new(reader: T, platform: Platform) -> Result<Self> {
        let mut reader = reader;
        reader.seek(std::io::SeekFrom::Start(0)).ok();
        let endian = if reader.read_ne::<u8>()? == 0x47 {
            Endian::Big
        } else {
            Endian::Little
        };
        reader.seek(std::io::SeekFrom::Start(0))?;

        let header = reader
            .read_type::<EXGeoHeader>(endian)
            .expect("Failed to read header");

        debug!(
            "Loaded EDB v{} (build date {}, size 0x{:x})",
            header.version, header.time, header.file_size
        );

        Ok(Self {
            reader,
            endian,
            platform,
            header,
        })
    }
}

impl<T: Read + Seek> Seek for DatabaseFile<T> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }
}

impl<T: Read + Seek> Read for DatabaseFile<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

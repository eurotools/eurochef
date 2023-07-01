use std::{
    io::{Read, Seek},
    mem::transmute,
};

use crate::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    versions::Platform,
    Hashcode,
};
use tracing::info;

use crate::error::Result;

pub trait DatabaseReader: Read + Seek {
    fn downcast_to_edbfile(&mut self) -> Option<&mut EdbFile>;
}

impl<R: Read + Seek + Sized> DatabaseReader for R {
    fn downcast_to_edbfile(&mut self) -> Option<&mut EdbFile> {
        // Safety: as long as the safety marker is present, we are good to downcast
        unsafe {
            let ptr: *mut EdbFile = transmute(self as *mut _);

            // Check alignment and safety marker
            if (transmute::<_, usize>(ptr) & 0x7) == 0
                && (*ptr).safety_marker == EdbFile::SAFETY_MARKER
            {
                Some(transmute(ptr))
            } else {
                None
            }
        }
    }
}

pub struct EdbFile<'d> {
    /// Using a marker to allow for safe downcasting when access to the object is needed in
    safety_marker: u64,

    reader: &'d mut dyn DatabaseReader,
    pub endian: Endian,
    pub platform: Platform,
    pub header: EXGeoHeader,

    pub external_references: Vec<(Hashcode, Hashcode)>,
}

impl<'d> EdbFile<'d> {
    pub const SAFETY_MARKER: u64 = 0xDEADC0FF47654F6D;

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
            safety_marker: Self::SAFETY_MARKER,
            reader,
            endian,
            platform,
            header,
            external_references: vec![],
        })
    }

    pub fn add_reference(&mut self, file: Hashcode, reference: Hashcode) {
        if !self.external_references.contains(&(file, reference)) {
            self.external_references.push((file, reference))
        }
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

pub trait EdbReaderMethods {
    fn add_reference(&mut self, file: Hashcode, reference: Hashcode);
}

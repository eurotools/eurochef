use std::{
    io::{Read, Seek},
    mem::transmute,
};

use crate::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
    versions::Platform,
    Hashcode, HashcodeUtils,
};
use tracing::{info, warn};

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
            if (ptr as usize & 0x7) == 0 && (*ptr).safety_marker == EdbFile::SAFETY_MARKER {
                Some(&mut *ptr)
            } else {
                if cfg!(debug_assertions) {
                    warn!("Couldn't verify EdbFile marker and alignment!");
                }

                None
            }
        }
    }
}

pub struct EdbFile {
    /// Using a marker to allow for safe downcasting when access to the object is needed in
    safety_marker: u64,

    reader: Box<dyn DatabaseReader>,
    pub endian: Endian,
    pub platform: Platform,
    pub header: EXGeoHeader,

    /// External hashcodes used by loaded objects
    pub external_references: Vec<(Hashcode, Hashcode)>,

    /// Hashcodes used by loaded objects that are located in this file
    pub internal_references: Vec<Hashcode>,
}

impl EdbFile {
    pub const SAFETY_MARKER: u64 = 0xDEADC0FF47654F6D;

    /// Resets the reader, tests endianness and reads the header
    pub fn new(reader: Box<dyn DatabaseReader>, platform: Platform) -> Result<Self> {
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

        if !(170..=263).contains(&version) {
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
            internal_references: vec![],
        })
    }

    pub fn add_reference(&mut self, file: Hashcode, reference: Hashcode) {
        if file == u32::MAX || reference.is_local() {
            self.add_reference_internal(reference);
        } else if !self.external_references.contains(&(file, reference)) {
            self.external_references.push((file, reference))
        }
    }

    pub fn add_reference_internal(&mut self, reference: u32) {
        if !self.internal_references.contains(&reference) {
            self.internal_references.push(reference);
        }
    }
}

impl Seek for EdbFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(pos)
    }
}

impl Read for EdbFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

pub trait EdbReaderMethods {
    fn add_reference(&mut self, file: Hashcode, reference: Hashcode);
}

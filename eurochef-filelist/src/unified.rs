use std::{
    collections::HashMap,
    io::{Read, Seek},
};

use anyhow::Result;
use binrw::{BinReaderExt, Endian};

use crate::{EXFileList4, EXFileList5};

pub struct UXFileList {
    /// `None` when using a single '.dat' file
    pub num_filelists: Option<u16>,
    pub build_type: Option<u16>,
    pub endian: Endian,
    pub files: Vec<(String, UXFileInfo)>,
}

pub struct UXFileInfo {
    pub addr: u32,
    pub filelist_num: Option<u32>,

    pub length: u32,
    pub hashcode: u32,
    pub version: u32,
    pub flags: u32,
    // ? Should we consider multiple filelocs?
}

// TODO: We should probably have our own error types, considering that this is a library
impl UXFileList {
    pub fn read<R>(reader: &mut R) -> Result<Self>
    where
        R: Read + Seek,
    {
        let marker: u8 = reader.read_ne()?;
        let endian = if marker == 0 {
            Endian::Big
        } else {
            Endian::Little
        };
        reader.seek(std::io::SeekFrom::Start(0))?;

        Self::read_endian(reader, endian)
    }

    pub fn read_endian<R>(reader: &mut R, endian: Endian) -> Result<Self>
    where
        R: Read + Seek,
    {
        let version: u32 = reader.read_type(endian)?;
        reader.seek(std::io::SeekFrom::Start(0))?;

        Ok(match version {
            4 => EXFileList4::read(reader)?.into(),
            5..=7 => EXFileList5::read(reader)?.into(),
            v => return Err(anyhow::anyhow!("Unsupported filelist version {}", v)),
        })
    }
}

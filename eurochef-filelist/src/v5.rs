use std::io::{Read, Seek};

use crate::path::unscramble_filename_v7;
use crate::structures::EXFileListHeader5;
use crate::unified::{UXFileInfo, UXFileList};

use anyhow::Result;
use binrw::{BinReaderExt, Endian};
use itertools::Itertools;

#[derive(Debug)]
pub struct EXFileList5 {
    pub endian: Endian,
    pub header: EXFileListHeader5,
    pub filenames: Vec<String>,
}

impl EXFileList5 {
    pub fn read<R>(reader: &mut R) -> Result<Self>
    where
        R: Read + Seek,
    {
        let endian = if reader.read_ne::<u8>()? != 0 {
            Endian::Little
        } else {
            Endian::Big
        };
        reader.seek(std::io::SeekFrom::Start(0))?;

        let mut res = Self {
            endian,
            header: reader.read_type(endian)?,
            filenames: vec![],
        };

        // TODO: RelPtr? (currently stuck in eurochef-edb)
        reader.seek(std::io::SeekFrom::Start(
            0x10 + res.header.filename_list_offset as u64,
        ))?;

        let base_offset = reader.stream_position()?;
        let mut filename_offsets = vec![];
        for i in 0..res.header.fileinfo.len() as u64 {
            filename_offsets.push(reader.read_type::<u32>(endian)? as u64 + base_offset + i * 4);
        }

        // FIXME: If the strings arent encoded linearly this will be a bit inefficient
        for (i, (start, end)) in filename_offsets
            .iter()
            .chain([res.header.filesize as u64].iter())
            .tuple_windows()
            .enumerate()
        {
            let size = end - start;
            let mut data = vec![0u8; size as usize];
            reader.seek(std::io::SeekFrom::Start(*start))?;
            reader.read(&mut data)?;

            if res.header.version >= 7 {
                unscramble_filename_v7(i as u32, &mut data);
            }

            let null_pos = data.iter().position(|&p| p == 0).unwrap_or(data.len());
            res.filenames
                .push(String::from_utf8_lossy(&data[0..null_pos]).to_string());
        }

        Ok(res)
    }
}

// TODO: Make a trait for filelists bundling both the read and from/into functions so that they can be used genericly
impl Into<UXFileList> for EXFileList5 {
    fn into(self) -> UXFileList {
        UXFileList {
            num_filelists: Some(self.header.num_filelists),
            build_type: Some(self.header.build_type),
            endian: self.endian,
            files: self
                .filenames
                .into_iter()
                .zip(self.header.fileinfo)
                .map(|(filename, info)| {
                    (
                        filename,
                        UXFileInfo {
                            addr: info.fileloc[0].addr,
                            filelist_num: Some(info.fileloc[0].filelist_num),
                            flags: info.flags,
                            hashcode: info.hashcode,
                            length: info.length,
                            version: info.version,
                        },
                    )
                })
                .collect(),
        }
    }
}

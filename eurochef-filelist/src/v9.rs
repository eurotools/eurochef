use std::io::{Read, Seek};

use crate::path::{unscramble_filename_v10, unscramble_filename_v7};
use crate::structures::EXFileListHeader9;
use crate::unified::{UXFileInfo, UXFileList};

use anyhow::Result;
use binrw::{BinReaderExt, Endian};
use itertools::Itertools;

#[derive(Debug)]
pub struct EXFileList9 {
    pub endian: Endian,
    pub header: EXFileListHeader9,
    pub filenames: Vec<String>,
}

impl EXFileList9 {
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

        filename_offsets.sort();
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
            reader.read_exact(&mut data)?;

            if res.header.version >= 10 {
                unscramble_filename_v10(i as u32, &mut data);
            } else {
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
impl From<EXFileList9> for UXFileList {
    fn from(val: EXFileList9) -> Self {
        UXFileList {
            num_filelists: Some(val.header.num_filelists),
            build_type: Some(val.header.build_type),
            endian: val.endian,
            files: val
                .filenames
                .into_iter()
                .zip(val.header.fileinfo)
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

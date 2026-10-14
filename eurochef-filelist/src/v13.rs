use std::io::{Read, Seek, SeekFrom};

use crate::structures::EXFileListHeader13;
use crate::unified::{UXFileInfo, UXFileList};

use anyhow::Result;
use binrw::{BinReaderExt, Endian};

/// v13 filelist (Disney Universe and later EngineX games). No embedded
/// filenames, no embedded addresses - see resolve_addresses for how files are
/// actually located in the data file.
#[derive(Debug)]
pub struct EXFileList13 {
    pub endian: Endian,
    pub header: EXFileListHeader13,
}

impl EXFileList13 {
    pub fn read<R>(reader: &mut R) -> Result<Self>
    where
        R: Read + Seek,
    {
        let endian = if reader.read_ne::<u8>()? != 0 {
            Endian::Little
        } else {
            Endian::Big
        };
        reader.seek(SeekFrom::Start(0))?;

        Ok(Self {
            endian,
            header: reader.read_type(endian)?,
        })
    }

    /// Files are packed back-to-back in the data file, each one padded up to
    /// the next 0x800-byte boundary. Every chunk type (GEOM, MUSX/audio
    /// banks, etc.) re-embeds its own catalog hashcode 4 bytes after its
    /// type-specific magic - audio/MUSX chunks store it little-endian even on
    /// big-endian platforms, so both interpretations are checked. Returns
    /// (addr, length) per entry, in catalog order; length is derived from the
    /// next entry's resolved start (or EOF for the last entry), since
    /// self-reported in-chunk sizes (e.g. GEOM's file_size) can overshoot the
    /// real on-disk boundary.
    pub fn resolve_addresses<R>(&self, data: &mut R) -> Result<Vec<(u32, u32)>>
    where
        R: Read + Seek,
    {
        const ALIGN: u64 = 0x800;

        let data_size = data.seek(SeekFrom::End(0))?;

        let mut positions = Vec::with_capacity(self.header.fileinfo.len());
        let mut cur: u64 = 0;
        for (i, info) in self.header.fileinfo.iter().enumerate() {
            let expected_hash = info.hashcode;
            let found = if i == 0 {
                0
            } else {
                let mut candidate = (cur + 1).div_ceil(ALIGN) * ALIGN;
                let mut found = None;
                let mut buf = [0u8; 4];
                while candidate + 8 <= data_size {
                    data.seek(SeekFrom::Start(candidate + 4))?;
                    data.read_exact(&mut buf)?;
                    if u32::from_be_bytes(buf) == expected_hash
                        || u32::from_le_bytes(buf) == expected_hash
                    {
                        found = Some(candidate);
                        break;
                    }
                    candidate += ALIGN;
                }
                found.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Could not locate data for entry {} (hashcode {:#x}) after {:#x}",
                        i,
                        expected_hash,
                        cur
                    )
                })?
            };
            positions.push(found);
            cur = found;
        }

        let mut result = Vec::with_capacity(positions.len());
        for (i, &pos) in positions.iter().enumerate() {
            let end = positions.get(i + 1).copied().unwrap_or(data_size);
            result.push((pos as u32, (end - pos) as u32));
        }

        Ok(result)
    }
}

impl From<EXFileList13> for UXFileList {
    fn from(val: EXFileList13) -> Self {
        UXFileList {
            num_filelists: Some(val.header.num_filelists),
            build_type: Some(val.header.build_type),
            endian: val.endian,
            files: val
                .header
                .fileinfo
                .iter()
                .map(|info| {
                    (
                        // extract.rs strips the first 3 chars of every filename
                        // (meant to drop an "x:\" drive prefix from real paths);
                        // pad with 3 dummy chars so the full hashcode survives.
                        format!("xx_{:08x}.bin", info.hashcode),
                        UXFileInfo {
                            addr: 0, // not stored - see EXFileList13::resolve_addresses
                            filelist_num: Some(0),
                            flags: info.flags,
                            hashcode: info.hashcode,
                            length: info.length,
                            version: 13,
                        },
                    )
                })
                .collect(),
        }
    }
}

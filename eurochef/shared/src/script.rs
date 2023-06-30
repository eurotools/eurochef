use std::{io::Seek, ops::Range};

use eurochef_edb::{
    binrw::{BinReaderExt, BinResult, Endian},
    common::EXRelPtr,
    edb::EdbFile,
    error::Result,
    header::EXGeoAnimScriptHeader,
    script::{EXGeoAnimScript, EXGeoAnimScriptControllerChannels, EXGeoAnimScriptControllerHeader},
};

use crate::hashcodes::Hashcode;

#[derive(Debug, Clone)]
pub enum UXGeoScriptCommandData {
    Entity { hashcode: Hashcode, file: Hashcode },
    Sound { hashcode: Hashcode },
    Particle { hashcode: Hashcode, file: Hashcode },
    EventType { event_type: Hashcode },
    SubScript { hashcode: Hashcode, file: Hashcode },
    Unknown { cmd: u8, data: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct UXGeoScriptCommand {
    pub start: i16,
    pub length: u16,
    pub thread: u8,
    pub parent_thread: u8,
    pub controller_index: u8,

    pub data: UXGeoScriptCommandData,
}

impl UXGeoScriptCommand {
    pub fn range(&self) -> Range<isize> {
        self.start as isize..self.length as isize
    }
}

#[derive(Debug, Clone)]
pub struct UXGeoScript {
    pub hashcode: Hashcode,
    pub framerate: f32,
    pub length: u32,
    pub num_threads: u32,

    pub commands: Vec<UXGeoScriptCommand>,
    pub controllers: Vec<EXGeoAnimScriptControllerHeader>,
}

impl UXGeoScript {
    pub fn read_all(edb: &mut EdbFile) -> Result<Vec<UXGeoScript>> {
        let header = edb.header.clone();
        let mut res = vec![];
        for c in &header.animscript_list {
            res.push(Self::read(c, edb)?);
        }

        Ok(res)
    }

    pub fn read(header: &EXGeoAnimScriptHeader, edb: &mut EdbFile) -> Result<UXGeoScript> {
        edb.seek(std::io::SeekFrom::Start(header.address as u64))?;
        let script = edb.read_type::<EXGeoAnimScript>(edb.endian)?;

        let mut commands = vec![];
        for c in script.commands {
            let data = match c.cmd {
                3 => UXGeoScriptCommandData::Entity {
                    hashcode: u32_from_index(&c.data, edb.endian, 8)?,
                    file: u32_from_index(&c.data, edb.endian, 4)?,
                },
                4 => UXGeoScriptCommandData::SubScript {
                    hashcode: u32_from_index(&c.data, edb.endian, 8)?,
                    file: u32_from_index(&c.data, edb.endian, 4)?,
                },
                5 => UXGeoScriptCommandData::Sound {
                    hashcode: u32_from_index(&c.data, edb.endian, 20)?,
                },
                6 => UXGeoScriptCommandData::Particle {
                    hashcode: u32_from_index(&c.data, edb.endian, 8)?,
                    file: u32_from_index(&c.data, edb.endian, 4)?,
                },
                11 => UXGeoScriptCommandData::EventType {
                    event_type: u32_from_index(&c.data, edb.endian, 0)?,
                },
                i => UXGeoScriptCommandData::Unknown {
                    cmd: i,
                    data: c.data,
                },
            };

            commands.push(UXGeoScriptCommand {
                start: c.start,
                length: c.length,
                thread: c.thread,
                parent_thread: c.parent_thread,
                controller_index: c.controller_index,
                data,
            });
        }

        let pos_saved = edb.stream_position()?;
        edb.seek(std::io::SeekFrom::Start(
            script.thread_controllers.offset_absolute(),
        ))?;

        let mut controllers = vec![];
        for _ in 0..script.thread_controller_count {
            let v: BinResult<EXRelPtr<EXGeoAnimScriptControllerHeader>> = edb.read_type(edb.endian);
            if let Ok(v) = v {
                controllers.push(v.data());
            } else {
                controllers.push(EXGeoAnimScriptControllerHeader {
                    controller_count: 0,
                    channel_count: 0,
                    ctrl_mask: 0,
                    ctrl_channel_mask: 0,
                    channels: EXGeoAnimScriptControllerChannels::default(),
                });
            }
        }

        edb.seek(std::io::SeekFrom::Start(pos_saved))?;

        Ok(UXGeoScript {
            hashcode: header.hashcode,
            framerate: script.frame_rate,
            length: script.length,
            num_threads: 0,
            commands,
            controllers,
        })
    }
}

fn u32_from_index(data: &[u8], endian: Endian, index: usize) -> anyhow::Result<u32> {
    Ok(match endian {
        Endian::Big => u32::from_be_bytes(data[index..index + 4].try_into()?),
        Endian::Little => u32::from_le_bytes(data[index..index + 4].try_into()?),
    })
}

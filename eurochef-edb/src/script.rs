use binrw::{binread, BinRead, BinResult, VecArgs};
use serde::Serialize;

use crate::common::{EXRelPtr, EXVector};

#[binread]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimScript {
    #[brw(assert(vtable.eq(&0x300)))]
    pub vtable: u32, // 0x0
    pub length: u32,                          // 0x4
    pub thread_count: u8,                     // 0x8
    pub timejump_count: u8,                   // 0x9
    pub script_flags: u16,                    // 0xa
    pub frame_rate: f32,                      // 0xc
    pub bounds_box: [EXVector; 2],            // 0x10
    pub unk30: u32,                           // 0x30
    pub thread_controllers: EXRelPtr,         // 0x34
    pub thread_info: EXRelPtr,                // 0x38
    pub unk3c: u16,                           // 0x3c
    pub thread_controller_channel_count: u16, // 0x3e
    pub used_controller_types: u32,           // 0x40

    #[br(parse_with = parse_commands)]
    pub commands: Vec<EXGeoAnimScriptCmd>,
}

#[binrw::parser(reader, endian)]
fn parse_commands() -> BinResult<Vec<EXGeoAnimScriptCmd>> {
    let mut res = Vec::new();
    let mut commands_left = 1024;
    loop {
        if commands_left == 0 {
            return Err(binrw::Error::AssertFail {
                pos: reader.stream_position()?,
                message: "Exceeded command limit".to_string(),
            });
        }

        let cmd = EXGeoAnimScriptCmd::read_options(reader, endian, ())?;
        res.push(cmd.clone());
        if cmd.cmd_size == 0 {
            break;
        }

        commands_left -= 1;
    }

    Ok(res)
}

#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimScriptCmd {
    pub cmd: u8,
    pub cmd_size: u8,
    /// Start frame
    pub cmd_frame: i16,
    pub data: Vec<u8>,

    pub start: i16,
    pub length: u16,
    pub thread: u8,
    pub parent_thread: u8,

    pub unk0: u8,
    pub unk1: i8,
}

impl BinRead for EXGeoAnimScriptCmd {
    type Args<'a> = ();
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let cmd = u8::read_options(reader, endian, ())?;
        let size = u8::read_options(reader, endian, ())?;
        let frame = i16::read_options(reader, endian, ())?;

        let (start, length, thread, parent_thread, unk0, unk1) = if cmd != 0x12 {
            <_>::read_options(reader, endian, ())?
        } else {
            (0, 0, 0, 0, 0, 0)
        };

        let data = if size == 0 {
            vec![]
        } else {
            <Vec<u8>>::read_options(
                reader,
                endian,
                VecArgs {
                    count: if cmd != 0x12 {
                        (size - 4 - 8) as usize
                    } else {
                        (size - 4) as usize
                    },
                    inner: (),
                },
            )?
        };

        Ok(Self {
            cmd,
            cmd_size: size,
            cmd_frame: frame,
            data,
            start,
            length,
            thread,
            parent_thread,
            unk0,
            unk1,
        })
    }
}

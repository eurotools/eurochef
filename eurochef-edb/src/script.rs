use binrw::{binread, BinRead, BinReaderExt, BinResult, VecArgs};
use serde::Serialize;
use tracing::warn;

use crate::common::{EXRelPtr, EXVector, EXVector3};

#[binread]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimScript {
    #[brw(assert(vtable.eq(&0x300)))]
    pub vtable: u32, // 0x0
    pub length: u32,                  // 0x4
    pub _unk8: u8,                    // 0x8
    pub timejump_count: u8,           // 0x9
    pub script_flags: u16,            // 0xa
    pub frame_rate: f32,              // 0xc
    pub bounds_box: [EXVector; 2],    // 0x10
    pub unk30: u32,                   // 0x30
    pub thread_controllers: EXRelPtr, // 0x34
    pub thread_info: EXRelPtr,        // 0x38
    pub thread_controller_count: u16, // 0x3c
    pub _unk3e: u16,                  // 0x3e
    pub used_controller_types: u32,   // 0x40

    #[br(parse_with = parse_commands)]
    pub commands: Vec<EXGeoAnimScriptCmd>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimScriptControllerHeader {
    pub controller_count: u16,
    pub channel_count: u16,
    pub ctrl_mask: u32,
    pub ctrl_channel_mask: u32,

    pub channels: EXGeoAnimScriptControllerChannels,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct EXGeoAnimScriptControllerChannels {
    pub time_0: Vec<(f32, f32)>,         // 0x1
    pub time_1: Vec<(f32, f32)>,         // 0x2
    pub vector_0: Vec<(f32, EXVector3)>, // 0x4
    pub quat_0: Vec<(f32, EXVector)>,    // 0x8
    pub vector_1: Vec<(f32, EXVector3)>, // 0x10
}

impl BinRead for EXGeoAnimScriptControllerHeader {
    type Args<'a> = ();

    #[allow(unused_braces)]
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        (): Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let controller_count = reader.read_type(endian)?;
        let channel_count = reader.read_type(endian)?;
        let ctrl_mask = reader.read_type(endian)?;
        let ctrl_channel_mask = reader.read_type(endian)?;

        let mut channels = EXGeoAnimScriptControllerChannels::default();

        if controller_count == 0 {
            return Ok(EXGeoAnimScriptControllerHeader {
                controller_count,
                channel_count,
                ctrl_mask,
                ctrl_channel_mask,
                channels,
            });
        }

        macro_rules! with_offset {
            ($offset:expr, $inner:tt) => {
                let pos_saved = reader.stream_position()?;
                reader.seek(std::io::SeekFrom::Start($offset))?;

                $inner

                reader.seek(std::io::SeekFrom::Start(pos_saved))?;
            };
        }

        if (ctrl_mask & 0x1) != 0 {
            let num_keyframes: i16 = reader.read_type(endian)?;
            let _unk1: u16 = reader.read_type(endian)?;
            let data_ptr: EXRelPtr = reader.read_type(endian)?;
            with_offset!(data_ptr.offset_absolute(), {
                channels.time_0 = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: num_keyframes.abs() as usize,
                        inner: (),
                    },
                )?
            });
        }

        if (ctrl_mask & 0x2) != 0 {
            let num_keyframes: i16 = reader.read_type(endian)?;
            let _unk1: u16 = reader.read_type(endian)?;
            let data_ptr: EXRelPtr = reader.read_type(endian)?;
            with_offset!(data_ptr.offset_absolute(), {
                channels.time_1 = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: num_keyframes.abs() as usize,
                        inner: (),
                    },
                )?;
            });
        }

        if (ctrl_mask & 0x4) != 0 {
            let num_keyframes: i16 = reader.read_type(endian)?;
            let _unk1: u16 = reader.read_type(endian)?;
            let data_ptr: EXRelPtr = reader.read_type(endian)?;
            with_offset!(data_ptr.offset_absolute(), {
                channels.vector_0 = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: num_keyframes.abs() as usize,
                        inner: (),
                    },
                )?;
            });
        }

        if (ctrl_mask & 0x8) != 0 {
            let num_keyframes: i16 = reader.read_type(endian)?;
            let _unk1: u16 = reader.read_type(endian)?;
            let data_ptr: EXRelPtr = reader.read_type(endian)?;
            with_offset!(data_ptr.offset_absolute(), {
                channels.quat_0 = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: num_keyframes.abs() as usize,
                        inner: (),
                    },
                )?;
            });
        }

        if (ctrl_mask & 0x10) != 0 {
            let num_keyframes: i16 = reader.read_type(endian)?;
            let _unk1: u16 = reader.read_type(endian)?;
            let data_ptr: EXRelPtr = reader.read_type(endian)?;
            with_offset!(data_ptr.offset_absolute(), {
                channels.vector_1 = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: num_keyframes.abs() as usize,
                        inner: (),
                    },
                )?;
            });
        }

        for i in 5..32 {
            if (ctrl_mask & (1 << i)) != 0 {
                warn!("Unknown anim script controller channel 0x{:x}", 1 << i);
            }
        }

        Ok(Self {
            controller_count,
            channel_count,
            ctrl_mask,
            ctrl_channel_mask,

            channels,
        })
    }
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

    pub controller_index: u8,
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
            controller_index: unk0,
            unk1,
        })
    }
}

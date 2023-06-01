#![allow(non_camel_case_types)]
use binrw::{binrw, BinRead, BinReaderExt};
use serde::Serialize;

use crate::{
    common::{EXRelPtr, EXVector},
    versions::Platform,
};

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32))]
// TODO: Format is slightly different on versions 248 and below
pub struct EXGeoBaseEntity {
    pub flags: u32,       // 0x4
    pub sort_value: u16,  // 0x8
    pub render_order: u8, // 0xa
    #[serde(skip)]
    _pad0: u8, // 0xb
    pub surface_area: f32, // 0xc
    pub bounds_box: [EXVector; 2], // 0x10
    _unk30: [u32; 4],     // 0x30
    #[brw(if(version > 221))]
    _unk40: [u32; 4],
    pub gdi_count: u16, // 0x50
    pub gdi_index: u16, // 0x52
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32, platform: Platform))]
// TODO(cohae): With how complex this struct is getting it might be time for a manual parser
pub struct EXGeoMeshEntity {
    #[brw(args(version))]
    pub base: EXGeoBaseEntity, // 0x0

    // TODO(cohae): All of these need to be read by eurochef-edb
    pub texture_list: EXRelPtr<EXGeoEntity_TextureList>, // 0x54
    pub tristrip_data: EXRelPtr,                         // 0x58 / Is a weird format on PS2
    pub vertex_data: EXRelPtr,                           // 0x5c / 0x60

    #[brw(if(platform != Platform::Ps2))]
    pub vertex_colors: Option<EXRelPtr>, // 0x60 / on ps2 this is included in tristrip_data
    #[brw(if(platform != Platform::Ps2))]
    pub _unk64: Option<EXRelPtr>, // 0x64 / not on ps2
    #[brw(if(platform != Platform::Ps2))]
    pub _unk68: Option<EXRelPtr>, // 0x68 / not on ps2

    pub index_data: EXRelPtr, // 0x6c / 0x64 on ps2

    pub _unk70: u32, // 0x70 / 0x64

    #[brw(if(platform == Platform::GameCube || platform == Platform::Wii))]
    _unk74: [u32; 2], // 0x74

    #[brw(if(platform == Platform::Ps2))]
    tristrip_count_ps2: u16, // 0x68
    #[brw(if(platform == Platform::Ps2))]
    vertex_count_ps2: u16, // 0x6a
    #[brw(if(platform == Platform::Ps2))]
    index_count_ps2: u16, // 0x6d

    #[brw(if(platform != Platform::Ps2))]
    tristrip_count_all: u32, // 0x74
    #[brw(if(platform != Platform::Ps2))]
    vertex_count_all: u32, // 0x78
    #[brw(if(platform != Platform::Ps2))]
    _unk7c_all: u32, // 0x7c
    #[brw(if(platform != Platform::Ps2))]
    index_count_all: u32, // 0x80

    #[br(calc = if platform == Platform::Ps2 { tristrip_count_ps2 as u32 } else { tristrip_count_all })]
    pub tristrip_count: u32,
    #[br(calc = if platform == Platform::Ps2 { vertex_count_ps2 as u32 } else { vertex_count_all })]
    pub vertex_count: u32,
    #[br(calc = if platform == Platform::Ps2 { 0 } else { _unk7c_all })]
    pub _unk7c: u32,
    #[br(calc = if platform == Platform::Ps2 { index_count_ps2 as u32 } else { index_count_all })]
    pub index_count: u32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32, _platform: Platform))]
pub struct EXGeoMapZoneEntity {
    #[brw(args(version))]
    pub base: EXGeoBaseEntity, // 0x0

    pub _unk54: u32,        // 0x54
    pub entity_refptr: u32, // 0x58
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32, platform: Platform))]
pub struct EXGeoSplitEntity {
    #[brw(args(version))]
    pub base: EXGeoBaseEntity, // 0x0

    // TODO(cohae): Older games have different limits, how do we handle that when writing files?
    #[brw(assert(entity_count.le(&512)))]
    pub entity_count: u32, // 0x54

    #[brw(if(version.ne(&213) && version.ne(&177)))]
    _unk58: u32,

    #[br(count = entity_count, args { inner: (version, platform) })]
    pub entities: Vec<EXRelPtr<EXGeoEntity>>, // 0x5c
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoEntity_TextureList {
    #[serde(skip)]
    pub texture_count: u16,

    #[br(count = texture_count)]
    pub textures: Vec<u16>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32, platform: Platform))]
pub struct EXGeoEntity_TriStrip {
    pub tricount: u32,
    pub texture_index: i32,

    pub min_index: u16,
    pub num_indices: u16,
    pub flags: u16,
    pub trans_type: u16,
    #[brw(if(version <= 252 && version != 248 || platform == Platform::Xbox360))]
    _unk10: u32,
}

#[derive(Debug, Serialize, Clone)]
pub enum EXGeoEntity {
    Mesh(EXGeoMeshEntity),
    Split(EXGeoSplitEntity),
    MapZone(EXGeoMapZoneEntity),
    Instance(EXGeoBaseEntity), // TODO(cohae): unfinished
    UnknownType(u32),
}

impl EXGeoEntity {
    pub fn base(&self) -> Option<&EXGeoBaseEntity> {
        match self {
            EXGeoEntity::Mesh(e) => Some(&e.base),
            EXGeoEntity::Split(e) => Some(&e.base),
            EXGeoEntity::MapZone(e) => Some(&e.base),
            EXGeoEntity::Instance(e) => Some(&e),
            EXGeoEntity::UnknownType(_e) => None,
        }
    }
}

impl BinRead for EXGeoEntity {
    type Args<'a> = (u32, Platform);

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let obj_type: u32 = reader.read_type(endian)?;

        Ok(match obj_type {
            0x601 => EXGeoEntity::Mesh(reader.read_type_args(endian, args)?),
            0x603 => EXGeoEntity::Split(reader.read_type_args(endian, args)?),
            0x606 => EXGeoEntity::Instance(reader.read_type_args(endian, (args.0,))?),
            0x608 => EXGeoEntity::MapZone(reader.read_type_args(endian, args)?),
            t @ 0x600..=0x6ff => EXGeoEntity::UnknownType(t),
            _ => {
                return Err(binrw::Error::NoVariantMatch {
                    pos: reader.stream_position()?,
                })
            }
        })
    }
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct Ps2TriData {
    pub uv: [f32; 2],
    pub index: u16,
    pub _unk2: u16,
    pub rgba: [u8; 4],
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct Ps2TriStrip {
    pub tricount: u16,      // [0]
    pub texture_index: u16, // [1]
    pub _unk2: u16,         // [2]
    pub _unk3: u16,         // [3]
    pub _unk4: u32,
    pub _unk5: u32,

    #[br(count = tricount + 2)]
    pub vertices: Vec<Ps2TriData>,
}

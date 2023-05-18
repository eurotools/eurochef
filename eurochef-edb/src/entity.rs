#![allow(non_camel_case_types)]
use binrw::{binrw, BinRead, BinReaderExt};
use serde::Serialize;

use crate::{
    common::{EXRelPtr, EXVector},
    error::EurochefError,
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
pub struct EXGeoMeshEntity {
    #[brw(args(version))]
    pub base: EXGeoBaseEntity, // 0x0

    // TODO(cohae): All of these need to be read by eurochef-edb
    pub texture_list: EXRelPtr<EXGeoEntity_TextureList>, // 0x54
    pub tristrip_data: EXRelPtr,                         // 0x58
    pub vertex_data: EXRelPtr,                           // 0x5c
    pub vertex_colors: EXRelPtr,                         // 0x60
    pub _unk64: EXRelPtr,                                // 0x64
    pub _unk68: EXRelPtr,                                // 0x68
    pub index_data: EXRelPtr,                            // 0x6c

    pub _unk70: u32, // 0x70

    #[brw(if(platform == Platform::GameCube || platform == Platform::Wii))]
    _unk74: [u32; 2], // 0x74

    pub tristrip_count: u32, // 0x74
    pub vertex_count: u32,   // 0x78
    pub _unk7c: u32,         // 0x7c
    pub index_count: u32,    // 0x80
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
            0x608 => EXGeoEntity::MapZone(reader.read_type_args(endian, args)?),
            t @ 0x600..=0x6ff => {
                return Err(binrw::Error::Custom {
                    pos: reader.stream_position()?,
                    err: Box::new(EurochefError::Unsupported(format!(
                        "EXGeoEntity type 0x{t:x} is not supported!"
                    ))),
                })
            }
            t => {
                panic!("Invalid object type 0x{t:x}!")
            }
        })
    }
}

use binrw::{binrw, BinRead, BinResult, VecArgs};
use serde::Serialize;

use crate::{
    array::EXRelArray,
    common::{EXRelPtr, EXVector},
};

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32))]
pub struct EXGeoBaseAnimSkin {
    pub object_type: u32, // 0x0
    pub bone_count: u32,  // 0x4

    // TODO(cohae): Probably wrong, just needed to get rid of 8 bytes
    #[brw(if(version.ne(&213) && version.ne(&221) && version.ne(&163) && version.ne(&174)))]
    pub _unkc: [u32; 2], // 0xc
    #[brw(if(version.ne(&213) && version.ne(&163) && version.ne(&174)))]
    pub bounds_box: [EXVector; 2], // 0x10
    pub _unk30: [u32; 4], // 0x30

    #[br(count = bone_count)]
    pub rot_data: EXRelPtr<Vec<EXVector>>, // 0x40
    #[br(count = bone_count)]
    pub rot_data_relative: EXRelPtr<Vec<EXVector>>, // 0x44

    #[br(count = bone_count)]
    pub hier_data: EXRelPtr<Vec<EXGeoAnimSkinHierData>>, // 0x48
    pub _unk4c: EXRelPtr<()>, // 0x4c
    #[brw(if(version.ne(&213) && version.ne(&163) && version.ne(&174)))]
    pub _unk50: [u32; 2], // 0x50
    pub _unk58: EXRelPtr<u16>, // 0x58
    pub _unk5c: EXRelPtr<()>, // 0x5c
    #[brw(if(version.ne(&163)))]
    pub _unk60: Option<EXRelArray<()>>, // 0x60
    pub entities: EXRelArray<EXGeoAnimSkinEntity>, // 0x68
    pub more_entities: EXRelArray<EXGeoAnimSkinEntity>, // 0x70, face-related entities?
    pub _unk78: EXRelArray<()>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimSkinEntity {
    skin_data_ptr: EXRelPtr,
    pub parts_count: u32,

    #[bw(assert(false), ignore)]
    #[br(parse_with(parse_late_skindata), args(&skin_data_ptr, parts_count))]
    pub skin_data: EXRelPtr<Vec<EXRelPtr<EXGeoAnimSkinUnkWeightData>>>,

    pub section_index: u32,
    pub entity_index: u32, // TODO(cohae): Add to reference list
    pub morph_index: i32,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimSkinUnkWeightData {
    unk0_count: u32,
    #[br(count = unk0_count)]
    #[bw(assert(false), ignore)]
    pub unk0: EXRelPtr<Vec<u8>>,
    pub unk1: EXRelPtr,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct EXGeoAnimSkinHierData {
    pub link_index: u16,
    pub max_index: u16,
    #[brw(pad_after(2))]
    pub flags: u16,
}

#[binrw::parser(reader, endian)]
fn parse_late_skindata(
    ptr: &EXRelPtr,
    length: u32,
) -> BinResult<EXRelPtr<Vec<EXRelPtr<EXGeoAnimSkinUnkWeightData>>>> {
    let pos_saved = reader.stream_position()?;
    reader.seek(std::io::SeekFrom::Start(ptr.offset_absolute()))?;

    let inner = <_>::read_options(
        reader,
        endian,
        VecArgs {
            count: length as usize,
            inner: (),
        },
    )?;
    reader.seek(std::io::SeekFrom::Start(pos_saved))?;

    Ok(EXRelPtr::new_with_offset(
        ptr.offset_relative(),
        ptr.offset_absolute(),
        inner,
    ))
}

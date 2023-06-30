use binrw::{binrw, BinRead, BinReaderExt, BinWrite, VecArgs};
use serde::Serialize;

use crate::{
    common::{EXVector2, EXVector3},
    entity::EXGeoMeshEntityData,
    versions::Platform,
};

#[derive(Debug, Serialize, Clone)]
pub struct UXGeoMeshVertex {
    pub pos: EXVector3,
    pub normal: EXVector3,
    pub uv: EXVector2,
}

#[derive(Debug, Serialize, Clone)]
pub struct EXGeoMeshEntity {
    pub data: EXGeoMeshEntityData, // 0x0

    pub texture_list: Vec<u16>,

    pub vertices: Vec<UXGeoMeshVertex>,
    pub vertex_colors: Vec<[u8; 4]>,
    pub indices: Vec<u16>,
    pub tristrips: Vec<EXGeoEntityTriStrip>,
    pub tristrips_gx: Vec<GxTriStrip>,
    pub tristrips_ps2: Vec<Ps2TriStrip>,
}

impl BinRead for EXGeoMeshEntity {
    type Args<'a> = (u32, Platform);

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        (version, platform): Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let data: EXGeoMeshEntityData = reader.read_type_args(endian, (version, platform))?;

        let indices = if platform.is_gx() {
            vec![]
        } else {
            reader.seek(std::io::SeekFrom::Start(data.index_data.offset_absolute()))?;

            if platform == Platform::Xbox360 {
                let data_magic = reader.read_type::<u32>(endian).unwrap();
                if data_magic != 0x0BADF001 {
                    return Err(binrw::Error::BadMagic {
                        pos: reader.stream_position()?,
                        found: Box::new(data_magic),
                    });
                }

                reader.read_type::<u32>(endian).unwrap();
            }

            (0..data.index_count)
                .map(|_| reader.read_type::<u16>(endian).unwrap())
                .collect()
        };

        let mut vertices = Vec::with_capacity(data.vertex_count as usize);
        reader.seek(std::io::SeekFrom::Start(
            data.vertex_data_offset.offset_absolute(),
        ))?;

        if platform == Platform::Xbox360 {
            let data_magic = reader.read_type::<u32>(endian).unwrap();
            if data_magic != 0x0BADF002 {
                return Err(binrw::Error::BadMagic {
                    pos: reader.stream_position()?,
                    found: Box::new(data_magic),
                });
            }

            reader.read_type::<u32>(endian).unwrap();
        }

        for _ in 0..data.vertex_count {
            if platform == Platform::GameCube || platform == Platform::Wii {
                let d = reader.read_type::<(EXVector3, u32)>(endian)?;
                vertices.push(UXGeoMeshVertex {
                    pos: d.0,
                    normal: [0f32, 0f32, 0f32],
                    uv: [0.5f32, 0.5f32],
                });
            } else {
                match version {
                    252 | 250 | 251 | 240 | 221 => {
                        let d = reader.read_type::<(EXVector3, u32, EXVector2)>(endian)?;
                        vertices.push(UXGeoMeshVertex {
                            pos: d.0,
                            normal: [0f32, 0f32, 0f32],
                            uv: d.2,
                        });
                    }
                    248 | 259 | 260 => {
                        if platform == Platform::Xbox360 {
                            let d = reader.read_type::<(EXVector3, f32, EXVector3, f32)>(endian)?;
                            vertices.push(UXGeoMeshVertex {
                                pos: d.0,
                                normal: d.2,
                                uv: [0.0, 0.0],
                            });
                        } else {
                            vertices.push(UXGeoMeshVertex {
                                pos: reader.read_type(endian)?,
                                normal: reader.read_type(endian)?,
                                uv: reader.read_type(endian)?,
                            });
                        }
                    }
                    _ => {
                        // TODO(cohae): This should be an error, not a panic
                        panic!(
                            "Vertex format for version {version} is not known yet, report to cohae!"
                        );
                    }
                }
            }
        }

        let mut vertex_colors: Vec<[u8; 4]> = Vec::with_capacity(data.vertex_count as usize);
        if let Some(vertex_colors_offset) = &data.vertex_color_offset {
            if [Platform::GameCube, Platform::Wii].contains(&platform) {
                for _ in 0..data.vertex_count {
                    vertex_colors.push([255, 255, 255, 255]);
                }
            } else {
                reader.seek(std::io::SeekFrom::Start(
                    vertex_colors_offset.offset_absolute(),
                ))?;

                if platform == Platform::Xbox360 {
                    let data_magic = reader.read_type::<u32>(endian).unwrap();
                    if data_magic != 0x0BADF003 {
                        return Err(binrw::Error::BadMagic {
                            pos: reader.stream_position()?,
                            found: Box::new(data_magic),
                        });
                    }

                    reader.read_type::<u32>(endian).unwrap();
                }

                for _ in 0..data.vertex_count {
                    let rgba: [u8; 4] = reader.read_type(endian)?;
                    match platform {
                        Platform::Xbox360 => {
                            vertex_colors.push([rgba[1], rgba[2], rgba[3], rgba[0]]);
                        }
                        _ => {
                            vertex_colors.push([rgba[2], rgba[1], rgba[0], rgba[3]]);
                        }
                    }
                }
            }
        }

        let mut tristrips: Vec<EXGeoEntityTriStrip> = vec![];
        let mut tristrips_gx: Vec<GxTriStrip> = vec![];
        let mut tristrips_ps2: Vec<Ps2TriStrip> = vec![];

        reader.seek(std::io::SeekFrom::Start(
            data.tristrip_data_offset.offset_absolute(),
        ))?;
        match platform {
            Platform::GameCube | Platform::Wii => {
                tristrips_gx = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: data.tristrip_count as usize,
                        inner: (),
                    },
                )?;
            }
            Platform::Ps2 => {
                tristrips_ps2 = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: data.tristrip_count as usize,
                        inner: (),
                    },
                )?;
            }
            _ => {
                tristrips = reader.read_type_args(
                    endian,
                    VecArgs {
                        count: data.tristrip_count as usize,
                        inner: (version, platform),
                    },
                )?;
            }
        }

        Ok(EXGeoMeshEntity {
            texture_list: data.texture_list.textures.clone(),
            vertices,
            vertex_colors,
            indices,
            tristrips,
            tristrips_gx,
            tristrips_ps2,
            data,
        })
    }
}

impl BinWrite for EXGeoMeshEntity {
    type Args<'a> = (u32, Platform);

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        todo!()
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

#[binrw]
#[derive(Debug, Serialize, Clone)]
pub struct GxTriStrip {
    pub unk1: u16,
    pub texture_index: u16,
    pub flags: u16,
    pub transparency: u16, // transparency?
    pub data_size: u32,
    pub unk3: u32,
    pub unk4: [u32; 4],

    #[br(count = data_size / 2)]
    pub indices: Vec<u16>,
}

#[binrw]
#[derive(Debug, Serialize, Clone)]
#[brw(import(version: u32, platform: Platform))]
pub struct EXGeoEntityTriStrip {
    pub tricount: u32,
    pub texture_index: i32,

    pub min_index: u16,
    pub num_indices: u16,
    pub flags: u16,
    pub trans_type: u16,
    #[brw(if(version <= 252 && version != 248 || platform == Platform::Xbox360))]
    pub _unk10: u32,
}

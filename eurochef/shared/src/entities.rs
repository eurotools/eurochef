use std::io::{Read, Seek};

use bytemuck::{Pod, Zeroable};
use eurochef_edb::{
    binrw::{BinReaderExt, Endian, VecArgs},
    common::{EXVector, EXVector2, EXVector3},
    entity::{EXGeoEntity, EXGeoEntity_TriStrip, GxTriStrip, Ps2TriStrip},
    versions::Platform,
};
use tracing::{error, warn};

#[derive(Debug, Clone, Copy)]
pub struct TriStrip {
    pub start_index: u32,
    pub index_count: u32,
    pub texture_index: u32,
    pub transparency: u16,
    pub flags: u16,
    pub tri_count: u32,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct UXVertex {
    pub pos: EXVector3,
    pub norm: EXVector3,
    pub uv: EXVector2,
    pub color: EXVector,
}

pub fn read_entity<R: Read + Seek>(
    ent: &EXGeoEntity,
    vertex_data: &mut Vec<UXVertex>,
    indices: &mut Vec<u32>,
    strips: &mut Vec<TriStrip>,
    endian: Endian,
    version: u32,
    platform: Platform,
    data: &mut R,
    depth_limit: u32,
    remove_transparent: bool,
    convert_strips: bool,
) -> anyhow::Result<()> {
    if depth_limit == 0 {
        anyhow::bail!("Entity recursion limit reached!");
    }

    match ent {
        EXGeoEntity::Split(split) => {
            for e in split.entities.iter() {
                read_entity(
                    e,
                    vertex_data,
                    indices,
                    strips,
                    endian,
                    version,
                    platform,
                    data,
                    depth_limit - 1,
                    remove_transparent,
                    convert_strips,
                )?;
            }
        }
        EXGeoEntity::Mesh(mesh) => {
            if platform == Platform::Ps2 {
                let mut tristrips: Vec<Ps2TriStrip> = vec![];
                data.seek(std::io::SeekFrom::Start(
                    mesh.tristrip_data.offset_absolute(),
                ))?;
                for _ in 0..mesh.tristrip_count {
                    tristrips.push(data.read_type(endian)?)
                }

                let mut vertices: Vec<(EXVector3, u32)> = vec![];
                data.seek(std::io::SeekFrom::Start(mesh.vertex_data.offset_absolute()))?;
                for _ in 0..mesh.vertex_count {
                    vertices.push(data.read_type(endian)?)
                }

                let textures = &mesh.texture_list.textures;
                for t in &tristrips {
                    let texture_index = if mesh.base.flags & 0x1 != 0 {
                        // Index from texture list instead of the "global" array
                        if t.texture_index < textures.len() as u16 {
                            textures[t.texture_index as usize] as i32
                        } else {
                            error!("Tried to get texture #{} from texture list, but list only has {} elements!", t.texture_index, textures.len());
                            -1
                        }
                    } else {
                        t.texture_index as i32
                    };

                    let vstart = vertex_data.len();
                    let mut index_count = 0;

                    for i in &t.vertices {
                        let index = i.index & 0x0fff;
                        let operation = (i.index >> 12) & 0xf;
                        match operation {
                            0 => {}
                            // Restart strip (generate degenerate triangle(s))
                            0x5 => {
                                indices.push(vertex_data.len() as u32 - 1);
                                indices.push(vertex_data.len() as u32);
                                index_count += 2;
                            }
                            n => warn!("Unknown tristrip op 0x{n:x}"),
                        };

                        indices.push(vertex_data.len() as u32);
                        index_count += 1;

                        let vert = &vertices[index as usize];
                        vertex_data.push(UXVertex {
                            pos: vert.0,
                            uv: i.uv,
                            color: [
                                i.rgba[0] as f32 / 255.0,
                                i.rgba[1] as f32 / 255.0,
                                i.rgba[2] as f32 / 255.0,
                                i.rgba[3] as f32 / 127.0,
                            ],
                            norm: [0.0; 3],
                        });
                    }

                    strips.push(TriStrip {
                        start_index: vstart as u32,
                        index_count: index_count,
                        texture_index: texture_index as u32,
                        transparency: 0,
                        flags: 0,
                        tri_count: index_count - 2,
                    });
                }

                return Ok(());
            }

            let vertex_offset = vertex_data.len() as u32;
            let mut vertex_colors: Vec<EXVector> = vec![];
            if let Some(vertex_colors_offset) = &mesh.vertex_colors {
                data.seek(std::io::SeekFrom::Start(
                    vertex_colors_offset.offset_absolute(),
                ))?;

                // TODO(cohae): 0BADF003 (assert) + data size
                if platform == Platform::Xbox360 {
                    for _ in 0..2 {
                        data.read_type::<u32>(endian).unwrap();
                    }
                }

                for _ in 0..mesh.vertex_count {
                    let rgba: [u8; 4] = data.read_type(endian)?;
                    match platform {
                        Platform::Xbox360 => {
                            vertex_colors.push([
                                rgba[1] as f32 / 255.0,
                                rgba[2] as f32 / 255.0,
                                rgba[3] as f32 / 255.0,
                                rgba[0] as f32 / 255.0,
                            ]);
                        }
                        // ! Currently handled during strip assembly
                        // Platform::GameCube | Platform::Wii => {
                        //     vertex_colors.push([
                        //         rgba[0] as f32 / 255.0,
                        //         rgba[1] as f32 / 255.0,
                        //         rgba[2] as f32 / 255.0,
                        //         rgba[3] as f32 / 255.0,
                        //     ]);
                        // }
                        _ => {
                            vertex_colors.push([
                                rgba[2] as f32 / 255.0,
                                rgba[1] as f32 / 255.0,
                                rgba[0] as f32 / 255.0,
                                rgba[3] as f32 / 255.0,
                            ]);
                        }
                    }
                }
            } else {
                for _ in 0..mesh.vertex_count {
                    vertex_colors.push([1.0, 1.0, 1.0, 1.0]);
                }
            }

            data.seek(std::io::SeekFrom::Start(mesh.vertex_data.offset_absolute()))?;

            // TODO(cohae): 0BADF002 (assert) + vertex count???
            if platform == Platform::Xbox360 {
                for _ in 0..2 {
                    data.read_type::<u32>(endian).unwrap();
                }
            }

            for i in 0..mesh.vertex_count {
                match version {
                    252 | 250 | 251 | 240 | 221 => {
                        if platform == Platform::GameCube || platform == Platform::Wii {
                            let d = data.read_type::<(EXVector3, u32)>(endian)?;
                            vertex_data.push(UXVertex {
                                pos: d.0,
                                norm: [0f32, 0f32, 0f32],
                                uv: [0.5f32, 0.5f32],
                                color: [0.5f32, 0.5f32, 0.5f32, 1f32],
                            });
                        } else {
                            let d = data.read_type::<(EXVector3, u32, EXVector2)>(endian)?;
                            vertex_data.push(UXVertex {
                                pos: d.0,
                                norm: [0f32, 0f32, 0f32],
                                uv: d.2,
                                color: vertex_colors[i as usize],
                            });
                        }
                    }
                    248 | 259 | 260 => {
                        if platform == Platform::Xbox360 {
                            let d = data.read_type::<(EXVector3, f32, EXVector3, f32)>(endian)?;
                            vertex_data.push(UXVertex {
                                pos: d.0,
                                norm: d.2,
                                uv: [0.0, 0.0],
                                color: vertex_colors[i as usize],
                            });
                        } else {
                            vertex_data.push(UXVertex {
                                pos: data.read_type(endian)?,
                                norm: data.read_type(endian)?,
                                uv: data.read_type(endian)?,
                                color: vertex_colors[i as usize],
                            });
                        }
                    }
                    _ => {
                        panic!(
                            "Vertex format for version {version} is not known yet, report to cohae!"
                        );
                    }
                }
            }

            if platform == Platform::Xbox360 {
                for i in 0..mesh.vertex_count as usize {
                    vertex_data[vertex_offset as usize + i].uv = data.read_type(endian)?;
                }
            }

            let textures = &mesh.texture_list.textures;

            let mut tristrips: Vec<EXGeoEntity_TriStrip> = vec![];
            let mut new_indices: Vec<u32> = vec![];
            if platform == Platform::GameCube || platform == Platform::Wii {
                data.seek(std::io::SeekFrom::Start(
                    mesh.tristrip_data.offset_absolute(),
                ))?;

                let gx_strips: Vec<GxTriStrip> = data.read_type_args(
                    endian,
                    VecArgs {
                        count: mesh.tristrip_count as usize,
                        inner: (),
                    },
                )?;

                // Move the vertices out of the main array, as we have to rebuild them
                let original_verts = vertex_data[vertex_offset as usize..].to_vec();
                vertex_data.drain(vertex_offset as usize..);
                for s in gx_strips {
                    struct GxIndex {
                        pos: u16,
                        _unk0: u16,
                        color: u16,
                        uv: u16,
                    }

                    let mut converted_indices = vec![];
                    let mut offset = 0;
                    while offset < s.indices.len() {
                        let h = s.indices[offset];
                        let face_count = s.indices[offset + 1];

                        if h != 0x98 {
                            break;
                        }
                        offset += 2;
                        let mut temp = vec![];
                        let chunk: &[[u16; 4]] = bytemuck::cast_slice(
                            s.indices[offset..offset + face_count as usize * 4].as_ref(),
                        );
                        for c in chunk {
                            temp.push(GxIndex {
                                pos: c[0],
                                _unk0: c[1],
                                color: c[2],
                                uv: c[3],
                            });
                        }
                        offset += face_count as usize * 4;

                        converted_indices.push(temp);
                    }

                    let mut index_count = 0;
                    let start_index = new_indices.len();
                    for cv in converted_indices.into_iter() {
                        if index_count != 0 {
                            new_indices.push(vertex_data.len() as u32 - 1 - vertex_offset);
                            new_indices.push(vertex_data.len() as u32 - vertex_offset);
                            index_count += 2;
                        }

                        for c in cv {
                            let original_vert = original_verts[c.pos as usize];

                            // TODO(cohae): The only way we can know the amount of vertex colors is by iterating through all indices. This is something for the entity handling rewrite.
                            let mut color = [0u8; 4];
                            data.seek(std::io::SeekFrom::Start(
                                mesh.vertex_colors.as_ref().unwrap().offset_absolute()
                                    + 4 * c.color as u64,
                            ))?;
                            data.read_exact(&mut color)?;

                            data.seek(std::io::SeekFrom::Start(
                                mesh.texture_coordinates.as_ref().unwrap().offset_absolute()
                                    + 4 * c.uv as u64,
                            ))?;
                            let uv: (i16, i16) = data.read_type(endian)?;

                            new_indices.push(vertex_data.len() as u32 - vertex_offset);
                            index_count += 1;

                            // FIXME(cohae): not actually index count, fix the structure. (there's probably more to this, check dbg file)
                            let uv_dividend = match (mesh.index_count >> 28) & 0b1111 {
                                0 => 65536.0,
                                1 => 32768.0,
                                2 => 16384.0, // Confirmed
                                3 => 8192.0,  // Confirmed
                                4 => 4096.0,  // Confirmed
                                5 => 2048.0,  // Confirmed
                                6 => 1024.0,
                                7 => 512.0, // Confirmed
                                _ => unreachable!(),
                            };

                            vertex_data.push(UXVertex {
                                pos: original_vert.pos,
                                norm: [0f32, 0f32, 0f32],
                                uv: [uv.0 as f32 / uv_dividend, uv.1 as f32 / uv_dividend],
                                color: [
                                    color[0] as f32 / 255.0,
                                    color[1] as f32 / 255.0,
                                    color[2] as f32 / 255.0,
                                    color[3] as f32 / 255.0,
                                ],
                            });
                        }
                    }

                    tristrips.push(EXGeoEntity_TriStrip {
                        tricount: index_count as u32 - 2,
                        texture_index: s.texture_index as i32,
                        min_index: start_index as u16,
                        num_indices: index_count as u16,
                        flags: s.flags,
                        trans_type: s.transparency,
                        _unk10: 0,
                    });
                }
            } else {
                data.seek(std::io::SeekFrom::Start(mesh.index_data.offset_absolute()))?;

                // TODO(cohae): 0BADF001
                if platform == Platform::Xbox360 {
                    for _ in 0..2 {
                        data.read_type::<u16>(endian).unwrap();
                    }
                }

                new_indices = (0..mesh.index_count)
                    .map(|_| data.read_type::<u16>(endian).unwrap() as u32)
                    .collect();

                data.seek(std::io::SeekFrom::Start(
                    mesh.tristrip_data.offset_absolute(),
                ))?;

                tristrips = data.read_type_args(
                    endian,
                    VecArgs {
                        count: mesh.tristrip_count as usize,
                        inner: (version, platform),
                    },
                )?;
            }

            let mut index_offset_local = 0;
            for t in tristrips {
                if t.tricount < 1 {
                    break;
                }

                if t.trans_type != 0 && remove_transparent {
                    index_offset_local += t.tricount + 2;
                    continue;
                }

                let texture_index = if mesh.base.flags & 0x1 != 0 {
                    // Index from texture list instead of the "global" array
                    if t.texture_index < textures.len() as i32 {
                        textures[t.texture_index as usize] as i32
                    } else {
                        error!("Tried to get texture #{} from texture list, but list only has {} elements!", t.texture_index, textures.len());
                        -1
                    }
                } else {
                    t.texture_index
                };

                if convert_strips {
                    strips.push(TriStrip {
                        start_index: indices.len() as u32,
                        index_count: t.tricount * 3,
                        texture_index: texture_index as u32,
                        transparency: t.trans_type,
                        flags: t.flags,
                        tri_count: t.tricount,
                    });

                    for i in
                        (index_offset_local as usize)..(index_offset_local + t.tricount) as usize
                    {
                        if (i - index_offset_local as usize) % 2 == 0 {
                            indices.extend([
                                vertex_offset + new_indices[i + 2] as u32,
                                vertex_offset + new_indices[i + 1] as u32,
                                vertex_offset + new_indices[i] as u32,
                            ])
                        } else {
                            indices.extend([
                                vertex_offset + new_indices[i] as u32,
                                vertex_offset + new_indices[i + 1] as u32,
                                vertex_offset + new_indices[i + 2] as u32,
                            ])
                        }
                    }
                    index_offset_local += t.tricount + 2;
                } else {
                    strips.push(TriStrip {
                        start_index: indices.len() as u32,
                        index_count: t.tricount + 2,
                        texture_index: texture_index as u32,
                        transparency: t.trans_type,
                        flags: t.flags,
                        tri_count: t.tricount,
                    });

                    indices.extend_from_slice(
                        &new_indices[(index_offset_local as usize)
                            ..(index_offset_local + t.tricount + 2) as usize]
                            .iter()
                            .map(|v| vertex_offset + v)
                            .collect::<Vec<u32>>(),
                    );

                    index_offset_local += t.tricount + 2;
                }
            }
        }
        EXGeoEntity::UnknownType(u) => {
            warn!("Unsupported entity type 0x{u:x}")
        }
        _ => {}
    }

    Ok(())
}

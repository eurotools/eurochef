use base64::Engine;
use eurochef_shared::entities::{TriStrip, UXVertex};
use gltf::json::{self as gjson, validation::Checked};
use std::collections::HashMap;

use super::entities::Transparency;

// pub fn write_glb<W: Write>(gltf: &gjson::Root, out: &mut W) -> anyhow::Result<()> {
//     let json_string = gjson::serialize::to_string(gltf).context("glTF serialization error")?;
//     let mut json_offset = json_string.len() as u32;
//     align_to_multiple_of_four(&mut json_offset);
//     let glb = gltf::binary::Glb {
//         header: gltf::binary::Header {
//             magic: *b"glTF",
//             version: 2,
//             length: json_offset,
//         },
//         bin: None,
//         json: Cow::Owned(json_string.into_bytes()),
//     };
//     glb.to_writer(out).context("glTF binary output error")?;

//     Ok(())
// }

// fn align_to_multiple_of_four(n: &mut u32) {
//     *n = (*n + 3) & !3;
// }

/// Creates a scene with a single mesh in it
pub fn create_mesh_scene(name: &str) -> gjson::Root {
    let node = gjson::Node {
        camera: None,
        children: None,
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: Some(gjson::Index::new(0)),
        name: Some(name.to_string()),
        rotation: None,
        scale: None,
        translation: None,
        skin: None,
        weights: None,
    };

    let mesh = gjson::Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        primitives: vec![],
        weights: None,
    };

    let sampler = gjson::texture::Sampler::default();
    gjson::Root {
        accessors: vec![],
        buffers: vec![],
        buffer_views: vec![],
        meshes: vec![mesh],
        nodes: vec![node],
        samplers: vec![sampler],
        scenes: vec![gjson::Scene {
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            nodes: vec![gjson::Index::new(0)],
        }],
        asset: gjson::Asset {
            generator: Some("Eurochef".to_string()),
            ..Default::default()
        },
        extensions_used: vec!["KHR_materials_pbrSpecularGlossiness".to_string()],
        ..Default::default()
    }
}

/// Constructs a primitive and adds it to the first mesh in the scene
pub fn add_mesh_to_scene(
    root: &mut gjson::Root,
    vertices: &[UXVertex],
    indices: &[u32],
    strips: &[TriStrip],
    use_normals: bool,
    texture_map: &HashMap<u32, (String, Transparency)>,
    file_hash: u32,
) {
    let vdata: &[u8] = bytemuck::cast_slice(vertices);
    let idata: &[u8] = bytemuck::cast_slice(indices);

    let (min, max) = bounding_coords(vertices);

    let vertex_buffer = gjson::Buffer {
        byte_length: vdata.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(create_data_uri(vdata)),
    };

    let vertex_buffer_index = root.buffers.len() as u32;
    root.buffers.push(vertex_buffer.clone());

    let vertex_buffer_view = gjson::buffer::View {
        buffer: gjson::Index::new(vertex_buffer_index),
        byte_length: vertex_buffer.byte_length,
        byte_offset: None,
        byte_stride: Some(std::mem::size_of::<UXVertex>() as u32),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Checked::Valid(gjson::buffer::Target::ArrayBuffer)),
    };

    let vertex_buffer_view_index = root.buffer_views.len() as u32;
    root.buffer_views.push(vertex_buffer_view);

    let index_buffer = gjson::Buffer {
        byte_length: idata.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(create_data_uri(idata)),
    };

    let index_buffer_index = root.buffers.len() as u32;
    root.buffers.push(index_buffer.clone());

    let positions = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(vertex_buffer_view_index)),
        byte_offset: 0,
        count: vertices.len() as u32,
        component_type: Checked::Valid(gjson::accessor::GenericComponentType(
            gjson::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Checked::Valid(gjson::accessor::Type::Vec3),
        min: Some(gjson::Value::from(Vec::from(min))),
        max: Some(gjson::Value::from(Vec::from(max))),
        name: None,
        normalized: false,
        sparse: None,
    };
    let normals = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(vertex_buffer_view_index)),
        byte_offset: (3 * std::mem::size_of::<f32>()) as u32,
        count: vertices.len() as u32,
        component_type: Checked::Valid(gjson::accessor::GenericComponentType(
            gjson::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Checked::Valid(gjson::accessor::Type::Vec3),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };
    let uvs = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(vertex_buffer_view_index)),
        byte_offset: (6 * std::mem::size_of::<f32>()) as u32,
        count: vertices.len() as u32,
        component_type: Checked::Valid(gjson::accessor::GenericComponentType(
            gjson::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Checked::Valid(gjson::accessor::Type::Vec2),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };
    let colors = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(vertex_buffer_view_index)),
        byte_offset: (8 * std::mem::size_of::<f32>()) as u32,
        count: vertices.len() as u32,
        component_type: Checked::Valid(gjson::accessor::GenericComponentType(
            gjson::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Checked::Valid(gjson::accessor::Type::Vec4),
        min: None,
        max: None,
        name: None,
        normalized: false,
        sparse: None,
    };

    let a_position_index = root.accessors.len() as u32;
    let a_normals_index = a_position_index + 1;
    let a_uvs_index = a_position_index + 2;
    let a_colors_index = a_position_index + 3;

    root.accessors.push(positions);
    root.accessors.push(normals);
    root.accessors.push(uvs);
    root.accessors.push(colors);

    let mut material_map: HashMap<u32, u32> = HashMap::new();
    // Restore material map
    for (i, m) in root.materials.iter().enumerate() {
        let msplit = m.name.as_ref().unwrap().split('_').next().unwrap();
        let mhashcode = u32::from_str_radix(msplit, 16).unwrap();
        material_map.insert(mhashcode, i as u32);
    }

    for t in strips {
        if let std::collections::hash_map::Entry::Vacant(e) = material_map.entry(t.texture_index) {
            let (img_uri, transparency) = texture_map
                .get(&t.texture_index)
                .cloned()
                .unwrap_or((format!("{:08x}.png", t.texture_index), Transparency::Opaque));

            root.images.push(gjson::Image {
                uri: Some(img_uri),
                buffer_view: None,
                extensions: None,
                extras: Default::default(),
                mime_type: None,
                name: Some(format!("{:08x}_{:08x}.png", t.texture_index, file_hash)),
            });

            root.textures.push(gjson::Texture {
                sampler: Some(gjson::Index::new(0)),
                extensions: None,
                extras: Default::default(),
                source: gjson::Index::new(root.images.len() as u32 - 1),
                name: None,
            });

            root.materials.push(gjson::Material {
                alpha_mode: Checked::Valid(if transparency != Transparency::Opaque {
                    match transparency {
                        Transparency::Opaque => gjson::material::AlphaMode::Opaque,
                        Transparency::Blend => gjson::material::AlphaMode::Blend,
                        Transparency::_Additive => gjson::material::AlphaMode::Blend,
                        Transparency::Cutout => gltf::material::AlphaMode::Mask,
                    }
                } else {
                    match t.transparency {
                        0 => gjson::material::AlphaMode::Opaque,
                        // 1 => Additive blending
                        // 2 => Reverse_subtract blending
                        _ => gjson::material::AlphaMode::Blend,
                    }
                }),
                pbr_metallic_roughness: gjson::material::PbrMetallicRoughness {
                    metallic_factor: gjson::material::StrengthFactor(0.),
                    roughness_factor: gjson::material::StrengthFactor(1.),
                    base_color_texture: Some(gjson::texture::Info {
                        index: gjson::Index::new(root.textures.len() as u32 - 1),
                        tex_coord: 0,
                        extensions: None,
                        extras: Default::default(),
                    }),
                    ..Default::default()
                },
                name: Some(format!("{:08x}_{:08x}.png", t.texture_index, file_hash)),
                extensions: Some(gjson::extensions::material::Material {
                    pbr_specular_glossiness: Some(
                        gjson::extensions::material::PbrSpecularGlossiness {
                            diffuse_texture: Some(gjson::texture::Info {
                                index: gjson::Index::new(root.textures.len() as u32 - 1),
                                tex_coord: 0,
                                extensions: None,
                                extras: Default::default(),
                            }),
                            specular_factor: gjson::extensions::material::PbrSpecularFactor([
                                0.0, 0.0, 0.0,
                            ]),
                            glossiness_factor: gjson::material::StrengthFactor(0.0),
                            ..Default::default()
                        },
                    ),
                }),
                double_sided: (t.flags & 0x40) != 0,
                ..Default::default()
            });

            let material_index = root.materials.len() as u32 - 1;
            e.insert(material_index);
        }

        let material_id = material_map.get(&t.texture_index).unwrap();

        let index_buffer_view = gjson::buffer::View {
            buffer: gjson::Index::new(index_buffer_index),
            byte_length: t.index_count * std::mem::size_of::<u32>() as u32,
            byte_offset: Some(t.start_index * std::mem::size_of::<u32>() as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            target: Some(Checked::Valid(gjson::buffer::Target::ElementArrayBuffer)),
        };
        root.buffer_views.push(index_buffer_view);

        let index_accessor = gjson::Accessor {
            buffer_view: Some(gjson::Index::new(root.buffer_views.len() as u32 - 1)),
            byte_offset: 0,
            count: t.index_count,
            component_type: Checked::Valid(gjson::accessor::GenericComponentType(
                gjson::accessor::ComponentType::U32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Checked::Valid(gjson::accessor::Type::Scalar),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        };
        root.accessors.push(index_accessor);

        let primitive = gjson::mesh::Primitive {
            attributes: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    Checked::Valid(gjson::mesh::Semantic::Positions),
                    gjson::Index::new(a_position_index),
                );
                if use_normals {
                    map.insert(
                        Checked::Valid(gjson::mesh::Semantic::Normals),
                        gjson::Index::new(a_normals_index),
                    );
                }
                map.insert(
                    Checked::Valid(gjson::mesh::Semantic::TexCoords(0)),
                    gjson::Index::new(a_uvs_index),
                );
                map.insert(
                    Checked::Valid(gjson::mesh::Semantic::Colors(0)),
                    gjson::Index::new(a_colors_index),
                );
                map
            },
            extensions: Default::default(),
            extras: Default::default(),
            indices: Some(gjson::Index::new(root.accessors.len() as u32 - 1)),
            material: Some(gjson::Index::new(*material_id)),
            mode: Checked::Valid(gjson::mesh::Mode::Triangles),
            targets: None,
        };

        root.meshes[0].primitives.push(primitive);
    }
}

fn bounding_coords(vertices: &[UXVertex]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];

    for v in vertices {
        let p = v.pos;
        for i in 0..3 {
            min[i] = f32::min(min[i], p[i]);
            max[i] = f32::max(max[i], p[i]);
        }
    }
    (min, max)
}

fn create_data_uri(data: &[u8]) -> String {
    let mut uri = "data:application/octet-stream;base64,".to_string();
    base64::engine::general_purpose::STANDARD.encode_string(data, &mut uri);
    uri
}

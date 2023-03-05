use base64::Engine;
use bytemuck::{Pod, Zeroable};
use eurochef_edb::common::{EXVector2, EXVector3};
use gltf::json::{self as gjson, validation::Checked};
use std::collections::HashMap;

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

pub struct TriStrip {
    pub start_index: u32,
    pub index_count: u32,
    pub texture_hash: u32,
}

pub fn dump_single_mesh_to_scene(
    vertices: &[UXVertex],
    indices: &[u32],
    strips: &[TriStrip],
    use_normals: bool,
    texture_map: &HashMap<u32, String>,
) -> gjson::Root {
    let vdata: &[u8] = bytemuck::cast_slice(vertices);
    let idata: &[u8] = bytemuck::cast_slice(indices);

    let (min, max) = bounding_coords(vertices);
    let vertex_buffer = gjson::Buffer {
        byte_length: vdata.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        // name: None,
        uri: Some(create_data_uri(vdata)),
    };

    let vertex_buffer_view = gjson::buffer::View {
        buffer: gjson::Index::new(0),
        byte_length: vertex_buffer.byte_length,
        byte_offset: None,
        byte_stride: Some(std::mem::size_of::<UXVertex>() as u32),
        extensions: Default::default(),
        extras: Default::default(),
        // name: None,
        target: Some(Checked::Valid(gjson::buffer::Target::ArrayBuffer)),
    };
    let index_buffer = gjson::Buffer {
        byte_length: idata.len() as u32,
        extensions: Default::default(),
        extras: Default::default(),
        // name: None,
        uri: Some(create_data_uri(idata)),
    };

    let positions = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(0)),
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
        // name: None,
        normalized: false,
        sparse: None,
    };
    let normals = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(0)),
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
        // name: None,
        normalized: false,
        sparse: None,
    };
    let uvs = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(0)),
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
        // name: None,
        normalized: false,
        sparse: None,
    };

    let sampler = gjson::texture::Sampler::default();

    let node = gjson::Node {
        camera: None,
        children: None,
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: Some(gjson::Index::new(0)),
        // name: None,
        rotation: None,
        scale: None,
        translation: None,
        skin: None,
        weights: None,
    };

    let mut root = gjson::Root {
        accessors: vec![positions, normals, uvs],
        buffers: vec![vertex_buffer, index_buffer],
        buffer_views: vec![vertex_buffer_view],
        meshes: vec![],
        nodes: vec![node],
        samplers: vec![sampler],
        scenes: vec![gjson::Scene {
            extensions: Default::default(),
            extras: Default::default(),
            // name: None,
            nodes: vec![gjson::Index::new(0)],
        }],
        asset: gjson::Asset {
            generator: Some("Eurochef".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut mesh = gjson::Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        // name: None,
        primitives: vec![],
        weights: None,
    };

    let mut material_map: HashMap<u32, u32> = HashMap::new();
    for t in strips {
        if !material_map.contains_key(&t.texture_hash) {
            let img_uri = texture_map.get(&t.texture_hash).map(|v| v.clone());
            root.images.push(gjson::Image {
                uri: Some(img_uri.unwrap_or(format!("{:08x}_frame0.png", t.texture_hash))),
                buffer_view: None,
                extensions: None,
                extras: Default::default(),
                mime_type: None,
            });

            root.textures.push(gjson::Texture {
                sampler: Some(gjson::Index::new(0)),
                extensions: None,
                extras: Default::default(),
                source: gjson::Index::new(root.images.len() as u32 - 1),
            });

            root.materials.push(gjson::Material {
                pbr_metallic_roughness: gjson::material::PbrMetallicRoughness {
                    base_color_texture: Some(gjson::texture::Info {
                        index: gjson::Index::new(root.textures.len() as u32 - 1),
                        tex_coord: 0,
                        extensions: None,
                        extras: Default::default(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            });

            let material_index = root.materials.len() as u32 - 1;
            material_map.insert(t.texture_hash, material_index);
        }

        let material_id = material_map.get(&t.texture_hash).unwrap();

        let index_buffer_view = gjson::buffer::View {
            buffer: gjson::Index::new(1),
            byte_length: t.index_count * std::mem::size_of::<u32>() as u32,
            byte_offset: Some(t.start_index * std::mem::size_of::<u32>() as u32),
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            // name: None,
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
            // name: None,
            normalized: false,
            sparse: None,
        };
        root.accessors.push(index_accessor);

        let primitive = gjson::mesh::Primitive {
            attributes: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    Checked::Valid(gjson::mesh::Semantic::Positions),
                    gjson::Index::new(0),
                );
                if use_normals {
                    map.insert(
                        Checked::Valid(gjson::mesh::Semantic::Normals),
                        gjson::Index::new(1),
                    );
                }
                map.insert(
                    Checked::Valid(gjson::mesh::Semantic::TexCoords(0)),
                    gjson::Index::new(2),
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

        mesh.primitives.push(primitive);
    }

    root.meshes.push(mesh);

    root
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct UXVertex {
    pub pos: EXVector3,
    pub norm: EXVector3,
    pub uv: EXVector2,
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

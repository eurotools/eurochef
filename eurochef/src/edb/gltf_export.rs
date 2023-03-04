use anyhow::Context;
use base64::Engine;
use bytemuck::{Pod, Zeroable};
use eurochef_edb::common::{EXVector2, EXVector3};
use gltf::json::{self as gjson, validation::Checked};
use std::{borrow::Cow, io::Write};

pub fn write_glb<W: Write>(gltf: &gjson::Root, out: &mut W) -> anyhow::Result<()> {
    let json_string = gjson::serialize::to_string(gltf).context("glTF serialization error")?;
    let mut json_offset = json_string.len() as u32;
    align_to_multiple_of_four(&mut json_offset);
    let glb = gltf::binary::Glb {
        header: gltf::binary::Header {
            magic: *b"glTF",
            version: 2,
            length: json_offset,
        },
        bin: None,
        json: Cow::Owned(json_string.into_bytes()),
    };
    glb.to_writer(out).context("glTF binary output error")?;

    Ok(())
}

fn align_to_multiple_of_four(n: &mut u32) {
    *n = (*n + 3) & !3;
}

pub fn dump_single_mesh_to_scene(vertices: &[UXVertex], indices: &[u32]) -> gjson::Root {
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

    let index_buffer_view = gjson::buffer::View {
        buffer: gjson::Index::new(1),
        byte_length: index_buffer.byte_length,
        byte_offset: None,
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        // name: None,
        target: Some(Checked::Valid(gjson::buffer::Target::ElementArrayBuffer)),
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
        byte_offset: (5 * std::mem::size_of::<f32>()) as u32,
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
    let index_accessor = gjson::Accessor {
        buffer_view: Some(gjson::Index::new(1)),
        byte_offset: 0,
        count: indices.len() as u32,
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

    let primitive = gjson::mesh::Primitive {
        attributes: {
            let mut map = std::collections::HashMap::new();
            map.insert(
                Checked::Valid(gjson::mesh::Semantic::Positions),
                gjson::Index::new(0),
            );
            // map.insert(
            //     Checked::Valid(gjson::mesh::Semantic::Normals),
            //     gjson::Index::new(1),
            // );
            map.insert(
                Checked::Valid(gjson::mesh::Semantic::TexCoords(0)),
                gjson::Index::new(2),
            );
            map
        },
        extensions: Default::default(),
        extras: Default::default(),
        indices: Some(gjson::Index::new(3)),
        material: None,
        mode: Checked::Valid(gjson::mesh::Mode::Triangles),
        targets: None,
    };

    let mesh = gjson::Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        // name: None,
        primitives: vec![primitive],
        weights: None,
    };

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

    gjson::Root {
        accessors: vec![positions, normals, uvs, index_accessor],
        buffers: vec![vertex_buffer, index_buffer],
        buffer_views: vec![vertex_buffer_view, index_buffer_view],
        meshes: vec![mesh],
        nodes: vec![node],
        scenes: vec![gjson::Scene {
            extensions: Default::default(),
            extras: Default::default(),
            // name: None,
            nodes: vec![gjson::Index::new(0)],
        }],
        ..Default::default()
    }
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

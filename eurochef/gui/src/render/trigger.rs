use eurochef_edb::map::EXGeoBaseDatum;
use genmesh::{
    generators::{Cube, Cylinder, IndexedPolygon, SharedVertex},
    Triangulate,
};
use glam::{Mat4, Quat, Vec3};
use glow::HasContext;

use super::{
    blend::{set_blending_mode, BlendMode},
    viewer::RenderContext,
};

pub struct LinkLineRenderer;

impl LinkLineRenderer {
    pub fn new(_gl: &glow::Context) -> Result<Self, String> {
        Ok(Self)
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        start: Vec3,
        end: Vec3,
        color: Vec3,
        scale: f32,
    ) {
        set_blending_mode(gl, BlendMode::None);
        unsafe {
            let shader = context.shaders.trigger_link;
            gl.line_width(3.0);
            gl.use_program(Some(shader));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_view").as_ref(),
                false,
                &context.uniforms.view.to_cols_array(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(shader, "u_start").as_ref(),
                &start.to_array(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(shader, "u_end").as_ref(),
                &end.to_array(),
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(shader, "u_time").as_ref(),
                context.uniforms.time,
            );

            gl.uniform_1_f32(gl.get_uniform_location(shader, "u_scale").as_ref(), scale);

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(shader, "u_color").as_ref(),
                &color.to_array(),
            );

            gl.draw_arrays(glow::LINES, 0, 2);
        }
    }
}

pub struct SelectCubeRenderer {
    buffers: (glow::Buffer, glow::VertexArray),
}

impl SelectCubeRenderer {
    const VERTEX_DATA: &'static [[f32; 3]] = &[
        [0.5, -0.5, 0.5],   // Bottom, NE (0)
        [0.5, -0.5, -0.5],  // Bottom, SE (1)
        [-0.5, -0.5, -0.5], // Bottom, SW (2)
        [-0.5, -0.5, 0.5],  // Bottom, NW (3)
        [0.5, 0.5, 0.5],    // Top, NE (4)
        [0.5, 0.5, -0.5],   // Top, SE (5)
        [-0.5, 0.5, -0.5],  // Top, SW (6)
        [-0.5, 0.5, 0.5],   // Top, NW (7)
    ];

    const INDEX_DATA: &'static [u8] = &[
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        Ok(Self {
            buffers: Self::cube_data(gl),
        })
    }

    fn cube_data(gl: &glow::Context) -> (glow::Buffer, glow::VertexArray) {
        unsafe {
            let vertex_array = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vertex_array));
            let vertex_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(Self::VERTEX_DATA),
                glow::STATIC_DRAW,
            );
            let index_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(Self::INDEX_DATA),
                glow::STATIC_DRAW,
            );

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                std::mem::size_of::<[f32; 3]>() as i32,
                0,
            );

            (index_buffer, vertex_array)
        }
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        pos: Vec3,
        rotation: Quat,
        scale: f32,
    ) {
        set_blending_mode(gl, BlendMode::None);
        unsafe {
            let shader = context.shaders.select_cube;
            gl.line_width(1.0);
            gl.use_program(Some(shader));
            gl.bind_vertex_array(Some(self.buffers.1));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.buffers.0));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_view").as_ref(),
                false,
                &context.uniforms.view.to_cols_array(),
            );

            let model = Mat4::from_translation(pos)
                * Mat4::from_quat(rotation)
                * Mat4::from_scale(Vec3::splat(scale));
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_model").as_ref(),
                false,
                &model.to_cols_array(),
            );
            gl.uniform_4_f32(
                gl.get_uniform_location(shader, "u_color").as_ref(),
                0.913,
                0.547,
                0.125,
                1.0,
            );

            gl.draw_elements(
                glow::LINES,
                Self::INDEX_DATA.len() as _,
                glow::UNSIGNED_BYTE,
                0,
            );
        }
    }
}

pub struct CollisionDatumRenderer {
    buffers_cube: (glow::Buffer, glow::Buffer, glow::VertexArray, i32, i32),
    buffers_cylinder: (glow::Buffer, glow::Buffer, glow::VertexArray, i32, i32),
}

impl CollisionDatumRenderer {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        Ok(Self {
            buffers_cube: Self::load_cube_mesh(gl),
            buffers_cylinder: Self::load_cylinder_mesh(gl),
        })
    }

    fn load_cube_mesh(
        gl: &glow::Context,
    ) -> (glow::Buffer, glow::Buffer, glow::VertexArray, i32, i32) {
        let mesh = Cube::new();
        let vertices: Vec<[f32; 3]> = mesh.shared_vertex_iter().map(|v| v.pos.into()).collect();
        let mut indices = vec![];
        let mut indices_outline = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        for i in mesh.indexed_polygon_iter() {
            indices_outline.extend_from_slice(&[
                i.x as u16, i.y as u16, i.y as u16, i.z as u16, i.z as u16, i.w as u16, i.w as u16,
                i.x as u16,
            ]);
        }

        Self::load_mesh_data(gl, &vertices, &indices, &indices_outline)
    }

    fn load_cylinder_mesh(
        gl: &glow::Context,
    ) -> (glow::Buffer, glow::Buffer, glow::VertexArray, i32, i32) {
        let mesh = Cylinder::new(16);
        let vertices: Vec<[f32; 3]> = mesh
            .shared_vertex_iter()
            .map(|v| {
                let v: [f32; 3] = v.pos.into();
                [v[0], v[2], v[1]]
            })
            .collect();
        let mut indices = vec![];
        let mut indices_outline = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        for p in mesh.indexed_polygon_iter() {
            match p {
                genmesh::Polygon::PolyTri(tri) => {
                    indices_outline.extend_from_slice(&[
                        tri.x as u16,
                        tri.y as u16,
                        tri.y as u16,
                        tri.z as u16,
                        tri.z as u16,
                        tri.x as u16,
                    ]);
                }
                genmesh::Polygon::PolyQuad(quad) => {
                    indices_outline.extend_from_slice(&[
                        quad.x as u16,
                        quad.y as u16,
                        quad.y as u16,
                        quad.z as u16,
                        quad.z as u16,
                        quad.w as u16,
                        quad.w as u16,
                        quad.x as u16,
                    ]);
                }
            }
        }

        Self::load_mesh_data(gl, &vertices, &indices, &indices_outline)
    }

    fn load_mesh_data(
        gl: &glow::Context,
        vertices: &[[f32; 3]],
        indices: &[u16],
        indices_outline: &[u16],
    ) -> (glow::Buffer, glow::Buffer, glow::VertexArray, i32, i32) {
        unsafe {
            let vertex_array = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vertex_array));
            let vertex_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(vertices),
                glow::STATIC_DRAW,
            );
            let index_buffer_tris = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer_tris));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(indices),
                glow::STATIC_DRAW,
            );
            let index_buffer_lines = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer_lines));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(indices_outline),
                glow::STATIC_DRAW,
            );

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                std::mem::size_of::<[f32; 3]>() as i32,
                0,
            );

            (
                index_buffer_tris,
                index_buffer_lines,
                vertex_array,
                indices.len() as i32,
                indices_outline.len() as i32,
            )
        }
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        position: Vec3,
        rotation: Quat,
        collision: &EXGeoBaseDatum,
    ) {
        set_blending_mode(gl, BlendMode::None);
        unsafe {
            let shader = context.shaders.select_cube;
            gl.line_width(3.0);
            gl.use_program(Some(shader));

            let (ebo_tris, ebo_lines, vao, count_tris, count_lines) = match collision.dtype {
                0 => self.buffers_cube,
                // TODO(cohae): Might be a capsule
                3 => self.buffers_cylinder,
                _ => return,
            };

            let extents = match collision.dtype {
                3 => [
                    collision.extents[1],
                    collision.extents[0],
                    collision.extents[1],
                ],
                _ => collision.extents,
            };

            gl.bind_vertex_array(Some(vao));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_view").as_ref(),
                false,
                &context.uniforms.view.to_cols_array(),
            );

            let model = Mat4::from_translation(position + Vec3::from(collision.position))
                * Mat4::from_quat(rotation * Quat::from_array(collision.q))
                * Mat4::from_scale(extents.into());
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_model").as_ref(),
                false,
                &model.to_cols_array(),
            );
            gl.uniform_4_f32(
                gl.get_uniform_location(shader, "u_color").as_ref(),
                0.913,
                0.547,
                0.125,
                1.0,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo_lines));
            gl.draw_elements(glow::LINES, count_lines, glow::UNSIGNED_SHORT, 0);

            gl.uniform_4_f32(
                gl.get_uniform_location(shader, "u_color").as_ref(),
                0.913,
                0.547,
                0.125,
                0.3,
            );

            set_blending_mode(gl, BlendMode::Blend);
            gl.disable(glow::CULL_FACE);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo_tris));
            gl.draw_elements(glow::TRIANGLES, count_tris, glow::UNSIGNED_SHORT, 0);
            set_blending_mode(gl, BlendMode::None);
        }
    }
}

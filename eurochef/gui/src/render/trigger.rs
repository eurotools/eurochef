use eurochef_edb::map::EXGeoBaseDatum;
use glam::{Mat4, Quat, Vec3};
use glow::HasContext;

use super::{
    blend::{set_blending_mode, BlendMode},
    gl_helper, RenderUniforms,
};

pub struct LinkLineRenderer {
    shader: glow::Program,
}

impl LinkLineRenderer {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        Ok(Self {
            shader: gl_helper::compile_shader(
                gl,
                &[
                    (
                        glow::VERTEX_SHADER,
                        include_str!("../../assets/shaders/trigger_link.vert"),
                    ),
                    (
                        glow::FRAGMENT_SHADER,
                        include_str!("../../assets/shaders/trigger_link.frag"),
                    ),
                ],
                &[],
            )?,
        })
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        uniforms: &RenderUniforms,
        start: Vec3,
        end: Vec3,
        color: Vec3,
        scale: f32,
    ) {
        set_blending_mode(gl, BlendMode::None);
        unsafe {
            gl.line_width(3.0);
            gl.use_program(Some(self.shader));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.shader, "u_view").as_ref(),
                false,
                &uniforms.view.to_cols_array(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.shader, "u_start").as_ref(),
                &start.to_array(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.shader, "u_end").as_ref(),
                &end.to_array(),
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.shader, "u_time").as_ref(),
                uniforms.time,
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.shader, "u_scale").as_ref(),
                scale,
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.shader, "u_color").as_ref(),
                &color.to_array(),
            );

            gl.draw_arrays(glow::LINES, 0, 2);
        }
    }
}

pub struct SelectCubeRenderer {
    shader: glow::Program,
    buffers: (glow::Buffer, glow::VertexArray),
}

impl SelectCubeRenderer {
    const VERTEX_DATA: &[[f32; 3]] = &[
        [0.5, -0.5, 0.5],   // Bottom, NE (0)
        [0.5, -0.5, -0.5],  // Bottom, SE (1)
        [-0.5, -0.5, -0.5], // Bottom, SW (2)
        [-0.5, -0.5, 0.5],  // Bottom, NW (3)
        [0.5, 0.5, 0.5],    // Top, NE (4)
        [0.5, 0.5, -0.5],   // Top, SE (5)
        [-0.5, 0.5, -0.5],  // Top, SW (6)
        [-0.5, 0.5, 0.5],   // Top, NW (7)
    ];

    const INDEX_DATA: &[u8] = &[
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        Ok(Self {
            shader: gl_helper::compile_shader(
                gl,
                &[
                    (
                        glow::VERTEX_SHADER,
                        include_str!("../../assets/shaders/select_cube.vert"),
                    ),
                    (
                        glow::FRAGMENT_SHADER,
                        include_str!("../../assets/shaders/select_cube.frag"),
                    ),
                ],
                &[],
            )?,
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
        uniforms: &RenderUniforms,
        pos: Vec3,
        rotation: Quat,
        scale: f32,
    ) {
        set_blending_mode(gl, BlendMode::None);
        unsafe {
            gl.line_width(1.0);
            gl.use_program(Some(self.shader));
            gl.bind_vertex_array(Some(self.buffers.1));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.buffers.0));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.shader, "u_view").as_ref(),
                false,
                &uniforms.view.to_cols_array(),
            );

            let model = Mat4::from_translation(pos)
                * Mat4::from_quat(rotation)
                * Mat4::from_scale(Vec3::splat(scale));
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.shader, "u_model").as_ref(),
                false,
                &model.to_cols_array(),
            );
            gl.uniform_4_f32(
                gl.get_uniform_location(self.shader, "u_color").as_ref(),
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

pub struct CollisionCubeRenderer {
    shader: glow::Program,
    buffers: (glow::Buffer, glow::Buffer, glow::VertexArray),
}

impl CollisionCubeRenderer {
    const VERTEX_DATA: &[[f32; 3]] = &[
        // 3------0.
        // |`.    | `.
        // |  `2--+---1
        // |   |  |   |
        // 7---+--4.  |
        //  `. |    `.|
        //    `6------5
        [1.0, -1.0, 1.0],   // Bottom, NE (0)
        [1.0, -1.0, -1.0],  // Bottom, SE (1)
        [-1.0, -1.0, -1.0], // Bottom, SW (2)
        [-1.0, -1.0, 1.0],  // Bottom, NW (3)
        [1.0, 1.0, 1.0],    // Top, NE (4)
        [1.0, 1.0, -1.0],   // Top, SE (5)
        [-1.0, 1.0, -1.0],  // Top, SW (6)
        [-1.0, 1.0, 1.0],   // Top, NW (7)
    ];

    const INDEX_DATA_TRIANGLES: &[u8] = &[
        1, 0, 3, 2, 1, 3, // Bottom
        4, 5, 7, 5, 6, 7, // Top
        5, 1, 2, 5, 2, 6, // Front
        4, 0, 1, 4, 1, 5, // Right
        6, 2, 3, 6, 3, 7, // Left
        7, 3, 4, 4, 3, 0, // Back
    ];

    const INDEX_DATA_LINES: &[u8] = &[
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        Ok(Self {
            shader: gl_helper::compile_shader(
                gl,
                &[
                    (
                        glow::VERTEX_SHADER,
                        include_str!("../../assets/shaders/select_cube.vert"),
                    ),
                    (
                        glow::FRAGMENT_SHADER,
                        include_str!("../../assets/shaders/select_cube.frag"),
                    ),
                ],
                &[],
            )?,
            buffers: Self::cube_data(gl),
        })
    }

    fn cube_data(gl: &glow::Context) -> (glow::Buffer, glow::Buffer, glow::VertexArray) {
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
            let index_buffer_tris = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer_tris));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(Self::INDEX_DATA_TRIANGLES),
                glow::STATIC_DRAW,
            );
            let index_buffer_lines = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer_lines));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(Self::INDEX_DATA_LINES),
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

            (index_buffer_tris, index_buffer_lines, vertex_array)
        }
    }

    pub fn render(
        &self,
        gl: &glow::Context,
        uniforms: &RenderUniforms,
        position: Vec3,
        rotation: Quat,
        collision: &EXGeoBaseDatum,
    ) {
        set_blending_mode(gl, BlendMode::None);
        unsafe {
            gl.line_width(3.0);
            gl.use_program(Some(self.shader));
            gl.bind_vertex_array(Some(self.buffers.2));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.shader, "u_view").as_ref(),
                false,
                &uniforms.view.to_cols_array(),
            );

            let model = Mat4::from_translation(position + Vec3::from(collision.position))
                * Mat4::from_quat(rotation)
                * Mat4::from_scale(collision.extents.into());
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.shader, "u_model").as_ref(),
                false,
                &model.to_cols_array(),
            );
            gl.uniform_4_f32(
                gl.get_uniform_location(self.shader, "u_color").as_ref(),
                0.913,
                0.547,
                0.125,
                1.0,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.buffers.1));
            gl.draw_elements(
                glow::LINES,
                Self::INDEX_DATA_LINES.len() as _,
                glow::UNSIGNED_BYTE,
                0,
            );

            gl.uniform_4_f32(
                gl.get_uniform_location(self.shader, "u_color").as_ref(),
                0.913,
                0.547,
                0.125,
                0.3,
            );

            set_blending_mode(gl, BlendMode::Blend);
            gl.disable(glow::CULL_FACE);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.buffers.0));
            gl.draw_elements(
                glow::TRIANGLES,
                Self::INDEX_DATA_TRIANGLES.len() as _,
                glow::UNSIGNED_BYTE,
                0,
            );
            set_blending_mode(gl, BlendMode::None);
        }
    }
}

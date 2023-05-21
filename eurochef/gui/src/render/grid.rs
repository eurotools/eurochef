use glow::HasContext;

use crate::gl_helper;

use super::RenderUniforms;

pub struct GridRenderer {
    shader: glow::Program,
    size: i32,
}

impl GridRenderer {
    pub fn new(gl: &glow::Context, size: i32) -> Self {
        let shader_sources = [
            (
                glow::VERTEX_SHADER,
                include_str!("../../assets/shaders/grid.vert"),
            ),
            (
                glow::FRAGMENT_SHADER,
                include_str!("../../assets/shaders/grid.frag"),
            ),
        ];

        let shader = unsafe {
            let shader = gl_helper::compile_shader(gl, &shader_sources).unwrap();
            gl.use_program(Some(shader));
            gl.uniform_1_i32(gl.get_uniform_location(shader, "u_size").as_ref(), size);
            shader
        };

        Self { shader, size }
    }

    pub unsafe fn draw(&self, uniforms: &RenderUniforms, gl: &glow::Context) {
        gl.use_program(Some(self.shader));
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(self.shader, "u_view").as_ref(),
            false,
            &uniforms.view.to_cols_array(),
        );

        gl.draw_arrays(glow::LINES, 0, (self.size + 1) * 2 * 2); // 10 lines (+1), 2 points each, 2 sides (horizontal/vertical)
        gl.use_program(None);
    }
}

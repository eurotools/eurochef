use glow::HasContext;

use super::viewer::RenderContext;

#[derive(Clone)]
pub struct GridRenderer {
    size: i32,
}

impl GridRenderer {
    pub fn new(_gl: &glow::Context, size: i32) -> Self {
        Self { size }
    }

    pub unsafe fn draw(&self, context: &RenderContext, gl: &glow::Context) {
        let shader = context.shaders.grid;
        gl.use_program(Some(shader));
        gl.uniform_1_i32(
            gl.get_uniform_location(shader, "u_size").as_ref(),
            self.size,
        );
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(shader, "u_view").as_ref(),
            false,
            &context.uniforms.view.to_cols_array(),
        );

        gl.draw_arrays(glow::LINES, 0, (self.size + 1) * 2 * 2); // 10 lines (+1), 2 points each, 2 sides (horizontal/vertical)
        gl.use_program(None);
    }
}

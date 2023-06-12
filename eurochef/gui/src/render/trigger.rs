use glam::Vec3;
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

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.shader, "u_color").as_ref(),
                &color.to_array(),
            );

            gl.draw_arrays(glow::LINES, 0, 2);
        }
    }
}

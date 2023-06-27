use std::mem::transmute;

use glam::{IVec2, Mat4};
use glow::HasContext;

use super::viewer::RenderContext;

#[repr(u32)]
#[derive()]
pub enum PickBufferType {
    Trigger = 1,
    // Placement = 2,
}

#[derive(Clone)]
pub struct PickBuffer {
    pub framebuffer: Option<glow::Framebuffer>,
}

impl PickBuffer {
    pub fn new(_gl: &glow::Context) -> Self {
        Self { framebuffer: None }
    }

    pub fn init_draw(&mut self, gl: &glow::Context, size: IVec2) {
        // TODO(cohae): Don't recreate framebuffer on every click
        unsafe {
            // Create framebuffer object
            let framebuffer = gl
                .create_framebuffer()
                .expect("Failed to create framebuffer");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

            // Create color texture
            let color_texture = gl.create_texture().expect("Failed to create color texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(color_texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                size.x,
                size.y,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(color_texture),
                0,
            );

            // Check framebuffer completeness
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                panic!("Framebuffer is not complete");
            }

            // Unbind framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            self.framebuffer = Some(framebuffer);
            gl.viewport(0, 0, size.x, size.y);
            gl.depth_mask(true);
        }
    }

    pub fn draw<F>(
        &self,
        context: &RenderContext,
        gl: &glow::Context,
        model: Mat4,
        id: (PickBufferType, u32),
        draw_callback: F,
    ) where
        F: Fn(&glow::Context),
    {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);

            let shader = context.shaders.pickbuffer;
            gl.use_program(Some(shader));
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_view").as_ref(),
                false,
                &context.uniforms.view.to_cols_array(),
            );

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(shader, "u_model").as_ref(),
                false,
                &model.to_cols_array(),
            );

            gl.uniform_1_u32(
                gl.get_uniform_location(shader, "u_type").as_ref(),
                transmute(id.0),
            );

            gl.uniform_1_u32(gl.get_uniform_location(shader, "u_id").as_ref(), id.1);

            draw_callback(gl);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }
}

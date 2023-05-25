use glam::{Mat4, Vec2};
use glow::HasContext;

pub mod entity;
pub mod gl_helper;
pub mod grid;

pub struct RenderUniforms {
    pub view: Mat4,
}

impl RenderUniforms {
    pub fn new(orthographic: bool, orientation: Vec2, zoom: f32, aspect_ratio: f32) -> Self {
        let projection = if orthographic {
            glam::Mat4::orthographic_rh_gl(
                (aspect_ratio * -zoom) * 2.0,
                (-aspect_ratio * -zoom) * 2.0,
                (1.0 * -zoom) * 2.0,
                (-1.0 * -zoom) * 2.0,
                -50.0,
                2500.0,
            )
        } else {
            glam::Mat4::perspective_rh_gl(90.0_f32.to_radians(), aspect_ratio, 0.1, 1000.0)
        };

        let view = glam::Mat4::from_rotation_translation(
            glam::Quat::from_rotation_x(orientation.y) * glam::Quat::from_rotation_z(orientation.x),
            glam::vec3(0.0, 0.0, -zoom),
        );

        Self {
            view: projection * view,
        }
    }
}

pub unsafe fn start_render(gl: &glow::Context) {
    gl.depth_mask(true);
    gl.clear_depth_f32(1.0);
    gl.clear(glow::DEPTH_BUFFER_BIT);
    gl.cull_face(glow::FRONT);
    gl.enable(glow::DEPTH_TEST);
    gl.depth_func(glow::LEQUAL);
}

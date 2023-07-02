use eurochef_edb::{versions::Platform, Hashcode};
use eurochef_shared::entities::{TriStrip, UXVertex};
use glam::{Mat4, Quat, Vec2, Vec3};
use glow::HasContext;

use crate::entities::ProcessedEntityMesh;

use super::{
    blend::{set_blending_mode, BlendMode},
    viewer::RenderContext,
    RenderStore,
};

#[derive(Clone)]
pub struct EntityRenderer {
    mesh: Option<(usize, glow::VertexArray, glow::Buffer, Vec<TriStrip>)>,
    platform: Platform,
    flags: u32,
    pub file_hashcode: Hashcode,
    pub vertex_lighting: bool,
}

impl EntityRenderer {
    pub fn new(file_hashcode: Hashcode, platform: Platform) -> Self {
        Self {
            mesh: None,
            platform,
            flags: 0,
            vertex_lighting: true,
            file_hashcode,
        }
    }

    /// Returns the center of the model (average of all points)
    pub unsafe fn load_mesh(&mut self, gl: &glow::Context, mesh: &ProcessedEntityMesh) -> Vec3 {
        let ProcessedEntityMesh {
            vertex_data,
            indices,
            strips,
            flags,
        } = mesh;

        let bounding_box = mesh.bounding_box();
        let center = (bounding_box.0 + bounding_box.1) / 2.0;

        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));
        let vertex_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(vertex_data),
            glow::STATIC_DRAW,
        );
        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(indices),
            glow::STATIC_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(
            0,
            3,
            glow::FLOAT,
            false,
            std::mem::size_of::<UXVertex>() as i32,
            0,
        );

        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(
            1,
            3,
            glow::FLOAT,
            false,
            std::mem::size_of::<UXVertex>() as i32,
            3 * std::mem::size_of::<f32>() as i32,
        );

        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_f32(
            2,
            2,
            glow::FLOAT,
            false,
            std::mem::size_of::<UXVertex>() as i32,
            6 * std::mem::size_of::<f32>() as i32,
        );

        gl.enable_vertex_attrib_array(3);
        gl.vertex_attrib_pointer_f32(
            3,
            4,
            glow::FLOAT,
            false,
            std::mem::size_of::<UXVertex>() as i32,
            8 * std::mem::size_of::<f32>() as i32,
        );

        gl.bind_vertex_array(None);

        let mut strips_sorted = strips.to_vec();
        strips_sorted.sort_by(|a, b| a.transparency.cmp(&b.transparency));

        self.mesh = Some((indices.len(), vertex_array, index_buffer, strips_sorted));
        self.flags = *flags;

        center
    }

    unsafe fn init_draw(
        &self,
        gl: &glow::Context,
        shader: glow::Program,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        context: &RenderContext,
    ) {
        gl.use_program(Some(shader));
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(shader, "u_view").as_ref(),
            false,
            &context.uniforms.view.to_cols_array(),
        );

        let mut rotation = rotation;

        if (self.flags & 0x4) != 0 {
            rotation = context.uniforms.camera_rotation;
        }

        let model =
            Mat4::from_translation(position) * Mat4::from_quat(rotation) * Mat4::from_scale(scale);
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(shader, "u_model").as_ref(),
            false,
            &model.to_cols_array(),
        );

        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(shader, "u_normal").as_ref(),
            false,
            &(context.uniforms.view * model)
                .inverse()
                .transpose()
                .to_cols_array(),
        );

        gl.uniform_1_i32(gl.get_uniform_location(shader, "u_texture").as_ref(), 0);
    }

    pub fn get_shader(&self, context: &RenderContext) -> glow::Program {
        if self.vertex_lighting {
            context.shaders.entity_simple
        } else {
            context.shaders.entity_simple_unlit
        }
    }

    pub unsafe fn draw_both(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        time: f64,
        render_store: &RenderStore,
    ) {
        self.draw_opaque(gl, context, position, rotation, scale, time, render_store);
        gl.depth_mask(false);
        self.draw_transparent(gl, context, position, rotation, scale, time, render_store);
    }

    pub unsafe fn draw_opaque(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        time: f64,
        render_store: &RenderStore,
    ) {
        if let Some((_index_count, vertex_array, index_buffer, strips)) = self.mesh.as_ref() {
            // self.init_draw(
            //     gl,
            //     self.get_shader(context),
            //     position,
            //     rotation,
            //     scale,
            //     context,
            // );
            gl.bind_vertex_array(Some(*vertex_array));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*index_buffer));

            for t in strips
                .iter()
                .filter(|t| t.transparency == 0 && (t.flags & 0x8) == 0)
            {
                self.draw_strip(
                    gl,
                    self.get_shader(context),
                    t,
                    time,
                    render_store,
                    position,
                    rotation,
                    scale,
                    context,
                );
            }
        }
    }

    pub unsafe fn draw_transparent(
        &self,
        gl: &glow::Context,
        context: &RenderContext,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        time: f64,
        render_store: &RenderStore,
    ) {
        if let Some((_index_count, vertex_array, index_buffer, strips)) = self.mesh.as_ref() {
            // self.init_draw(
            //     gl,
            //     self.get_shader(context),
            //     position,
            //     rotation,
            //     scale,
            //     context,
            // );
            gl.bind_vertex_array(Some(*vertex_array));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*index_buffer));

            let shader = if self.vertex_lighting {
                context.shaders.entity_simple
            } else {
                context.shaders.entity_simple_unlit
            };
            for t in strips
                .iter()
                .filter(|t| t.transparency != 0 || (t.flags & 0x8) != 0)
            {
                self.draw_strip(
                    gl,
                    shader,
                    t,
                    time,
                    render_store,
                    position,
                    rotation,
                    scale,
                    context,
                );
            }
        }
    }

    unsafe fn draw_strip(
        &self,
        gl: &glow::Context,
        shader: glow::Program,
        t: &TriStrip,
        time: f64,
        render_store: &RenderStore,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        context: &RenderContext,
    ) {
        // For stripflags (EX):
        // 0x1 - transparent / vertex blended?
        // 0x2 - ?
        // 0x4 - additive
        // 0x8 - ? kinda like 0x1 but not really
        // 0x10 - invisible
        // 0x20 - ?
        // 0x40 - double sided (disable culling)
        // 0x80 - seems to be used for anything that's not transparent OR using vertex color transparency stuck to the world
        // 0x100 - ? (used by godrays in gforce)
        // 0x200 - mostly additive surfaces, but not all
        // 0x400 - used by everything that isn't a floor
        // 0x800 - unused?
        // 0x1000 - unused?
        // 0x2000 - unused?
        // 0x4000 - unused?
        // 0x8000 - unused?

        // Hide what is hidden
        if (t.flags & 0x10) != 0 {
            return;
        }

        let mut shader = shader;

        let mut transparency = match t.transparency & 0xff {
            2 => BlendMode::ReverseSubtract,
            1 => BlendMode::Additive,
            0 | _ => BlendMode::None,
        };

        if ((t.flags & 0x8) != 0 || (t.flags & 0x1) != 0) && transparency == BlendMode::None {
            transparency = BlendMode::Blend;
        }

        if (t.flags & 0x40) != 0 {
            gl.disable(glow::CULL_FACE);
        } else {
            // TODO(cohae): PS2/GX Strips aren't built with the correct winding order
            match self.platform {
                Platform::GameCube | Platform::Wii | Platform::Ps2 => {
                    gl.disable(glow::CULL_FACE);
                }
                _ => {
                    gl.enable(glow::CULL_FACE);
                }
            }
        }

        let mut scroll = Vec2::ZERO;

        gl.active_texture(glow::TEXTURE0);
        if let Some((_, tex)) =
            render_store.get_texture_by_index(self.file_hashcode, t.texture_index as usize)
        {
            let frametime_scale = tex.frame_count as f32 / tex.frames.len() as f32;
            let frame_time = (1. / tex.framerate as f32) * frametime_scale;

            scroll = tex.scroll * time as f32;

            if tex.frames.len() > 0 {
                gl.bind_texture(
                    glow::TEXTURE_2D,
                    Some(tex.frames[(time as f32 / frame_time) as usize % tex.frames.len()]),
                );
            } else {
                gl.bind_texture(glow::TEXTURE_2D, None);
            }
            if (((tex.flags >> 0x18) >> 5) & 0b11) != 0 && (t.flags & 0x8) == 0 {
                transparency = BlendMode::Cutout;
            }

            // Environment texture
            if (tex.flags & 0x30000) != 0 {
                shader = context.shaders.entity_simple_matcap;
            }
        } else {
            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        self.init_draw(gl, shader, position, rotation, scale, context);

        gl.uniform_2_f32(
            gl.get_uniform_location(shader, "u_scroll").as_ref(),
            scroll.x,
            scroll.y,
        );

        set_blending_mode(gl, transparency);

        gl.uniform_1_f32(
            gl.get_uniform_location(shader, "u_cutoutThreshold")
                .as_ref(),
            if transparency == BlendMode::Cutout {
                0.5
            } else {
                0.0
            },
        );

        gl.draw_elements(
            glow::TRIANGLE_STRIP,
            (t.tri_count + 2) as i32,
            glow::UNSIGNED_INT,
            t.start_index as i32 * std::mem::size_of::<u32>() as i32,
        );
    }
}

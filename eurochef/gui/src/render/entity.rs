use eurochef_shared::entities::{TriStrip, UXVertex};
use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};
use glow::HasContext;

use crate::{entities::ProcessedEntityMesh, entity_frame::RenderableTexture};

use super::{gl_helper, RenderUniforms};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BlendMode {
    None,
    Cutout,
    Blend,
    Additive,
    ReverseSubtract,
}

pub struct EntityRenderer {
    mesh_shader: glow::Program,
    mesh: Option<(usize, glow::VertexArray, glow::Buffer, Vec<TriStrip>)>,

    textures: Vec<RenderableTexture>,

    pub orthographic: bool,
}

impl EntityRenderer {
    pub fn new(gl: &glow::Context, textures: Vec<RenderableTexture>) -> Self {
        Self {
            mesh_shader: unsafe { Self::create_mesh_program(gl).unwrap() },
            mesh: None,
            orthographic: false,
            textures,
        }
    }

    /// Returns the center of the model (average of all points)
    pub unsafe fn load_mesh(&mut self, gl: &glow::Context, mesh: &ProcessedEntityMesh) -> Vec3 {
        let ProcessedEntityMesh {
            vertex_data,
            indices,
            strips,
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

        center
    }

    unsafe fn create_mesh_program(gl: &glow::Context) -> Result<glow::Program, String> {
        let shader_sources = [
            (
                glow::VERTEX_SHADER,
                include_str!("../../assets/shaders/entity.vert"),
            ),
            (
                glow::FRAGMENT_SHADER,
                include_str!("../../assets/shaders/entity.frag"),
            ),
        ];

        gl_helper::compile_shader(gl, &shader_sources)
    }

    unsafe fn init_draw(&self, gl: &glow::Context, mesh_center: Vec3, uniforms: &RenderUniforms) {
        gl.use_program(Some(self.mesh_shader));
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(self.mesh_shader, "u_view").as_ref(),
            false,
            &uniforms.view.to_cols_array(),
        );

        let model = Mat4::from_translation(-mesh_center.zxy());
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(self.mesh_shader, "u_model")
                .as_ref(),
            false,
            &model.to_cols_array(),
        );

        gl.uniform_1_i32(
            gl.get_uniform_location(self.mesh_shader, "u_texture")
                .as_ref(),
            0,
        );
    }

    pub unsafe fn draw_both(
        &self,
        gl: &glow::Context,
        uniforms: &RenderUniforms,
        mesh_center: Vec3,
        time: f64,
    ) {
        self.draw_opaque(gl, uniforms, mesh_center, time);
        self.draw_transparent(gl, uniforms, mesh_center, time);
    }

    pub unsafe fn draw_opaque(
        &self,
        gl: &glow::Context,
        uniforms: &RenderUniforms,
        mesh_center: Vec3,
        time: f64,
    ) {
        if let Some((_index_count, vertex_array, index_buffer, strips)) = self.mesh.as_ref() {
            self.init_draw(gl, mesh_center, uniforms);
            gl.bind_vertex_array(Some(*vertex_array));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*index_buffer));

            for t in strips.iter().filter(|t| t.transparency == 0) {
                self.draw_strip(gl, t, time);
            }
        }
    }

    pub unsafe fn draw_transparent(
        &self,
        gl: &glow::Context,
        uniforms: &RenderUniforms,
        mesh_center: Vec3,
        time: f64,
    ) {
        if let Some((_index_count, vertex_array, index_buffer, strips)) = self.mesh.as_ref() {
            self.init_draw(gl, mesh_center, uniforms);
            gl.bind_vertex_array(Some(*vertex_array));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*index_buffer));

            for t in strips.iter().filter(|t| t.transparency != 0) {
                self.draw_strip(gl, t, time);
            }
        }
    }

    unsafe fn draw_strip(&self, gl: &glow::Context, t: &TriStrip, time: f64) {
        // TODO(cohae): Transparency seems broken on newer games
        let mut transparency = match t.transparency & 0xff {
            2 => BlendMode::ReverseSubtract,
            1 => BlendMode::Additive,
            0 | _ => BlendMode::None,
        };

        if (t.flags & 0x8) != 0 && (t.flags & 0x1) == 0 {
            transparency = BlendMode::Blend;
        }

        if (t.flags & 0x40) != 0 {
            gl.disable(glow::CULL_FACE);
        } else {
            gl.enable(glow::CULL_FACE);
        }

        let mut scroll = Vec2::ZERO;

        gl.active_texture(glow::TEXTURE0);
        if (t.texture_index as usize) < self.textures.len() {
            let tex = &self.textures[t.texture_index as usize];
            // Cubemap texture
            if (tex.flags & 0x30000) != 0 {
                return;
            }

            let frametime_scale = tex.frame_count as f32 / tex.frames.len() as f32;
            let frame_time = (1. / tex.framerate as f32) * frametime_scale;

            scroll = tex.scroll * time as f32;

            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(tex.frames[(time as f32 / frame_time) as usize % tex.frames.len()]),
            );
            if (((tex.flags >> 0x18) >> 5) & 0b11) != 0 {
                transparency = BlendMode::Cutout;
            }
        } else {
            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        // Skip transparent surfaces on newer games for now
        if t.transparency > 0xff {
            return;
        }

        gl.uniform_2_f32(
            gl.get_uniform_location(self.mesh_shader, "u_scroll")
                .as_ref(),
            scroll.x,
            scroll.y,
        );

        self.set_blending_mode(gl, transparency);

        gl.uniform_1_f32(
            gl.get_uniform_location(self.mesh_shader, "u_cutoutThreshold")
                .as_ref(),
            if transparency == BlendMode::Cutout {
                0.5
            } else {
                0.0
            },
        );

        gl.draw_elements(
            glow::TRIANGLES,
            t.index_count as i32,
            glow::UNSIGNED_INT,
            t.start_index as i32 * std::mem::size_of::<u32>() as i32,
        );
    }

    fn set_blending_mode(&self, gl: &glow::Context, blend: BlendMode) {
        unsafe {
            match blend {
                BlendMode::None | BlendMode::Cutout => {
                    gl.disable(glow::BLEND);
                }
                _ => gl.enable(glow::BLEND),
            }

            match blend {
                BlendMode::Cutout => {
                    gl.enable(glow::SAMPLE_ALPHA_TO_COVERAGE);
                }
                _ => gl.disable(glow::SAMPLE_ALPHA_TO_COVERAGE),
            }

            match blend {
                BlendMode::Blend | BlendMode::Additive | BlendMode::ReverseSubtract => {
                    let blend_src = match blend {
                        BlendMode::Blend => glow::SRC_ALPHA,
                        BlendMode::Additive => glow::SRC_ALPHA,
                        BlendMode::ReverseSubtract => glow::SRC_ALPHA,
                        _ => unreachable!(),
                    };

                    let blend_dst = match blend {
                        BlendMode::Blend => glow::ONE_MINUS_SRC_ALPHA,
                        BlendMode::Additive => glow::ONE,
                        BlendMode::ReverseSubtract => glow::ONE,
                        _ => unreachable!(),
                    };

                    let blend_func = match blend {
                        BlendMode::Blend => glow::FUNC_ADD,
                        BlendMode::Additive => glow::FUNC_ADD,
                        BlendMode::ReverseSubtract => glow::FUNC_REVERSE_SUBTRACT,
                        _ => unreachable!(),
                    };

                    gl.blend_equation(blend_func);
                    gl.blend_func(blend_src, blend_dst);
                }
                _ => {}
            }
        }
    }
}

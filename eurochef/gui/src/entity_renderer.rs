use std::sync::{Arc, Mutex};

use eurochef_shared::entities::{TriStrip, UXVertex};
use glam::{DVec3, Mat4, Vec3, Vec3Swizzles};
use glow::HasContext;

use crate::{entities::ProcessedEntityMesh, gl_helper};

pub struct EntityFrame {
    pub hashcode: u32,

    renderer: Arc<Mutex<EntityRenderer>>,
    orientation: egui::Vec2,
    origin: Vec3,
    zoom: f32,

    mesh_center: Vec3,
    // model_origin: Vec3,
}

impl EntityFrame {
    pub fn new(gl: &glow::Context, hashcode: u32, mesh: &ProcessedEntityMesh) -> Self {
        let mut s = Self {
            hashcode,
            renderer: Arc::new(Mutex::new(EntityRenderer::new(gl))),
            orientation: egui::vec2(0., -1.),
            origin: Vec3::ZERO,
            zoom: 1.0,
            mesh_center: Vec3::ZERO,
        };

        unsafe {
            s.mesh_center = s.renderer.lock().unwrap().load_mesh(gl, mesh);
        }

        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, gl: Arc<glow::Context>) {
        ui.checkbox(
            &mut self.renderer.lock().unwrap().orthographic,
            "Orthographic",
        );
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        if response.dragged_by(egui::PointerButton::Middle) {
            self.orientation += response.drag_delta() * 0.005;
        }

        // if response.dragged_by(egui::PointerButton::Secondary) {
        //     self.pan_camera(response.drag_delta() * 0.015);
        // }

        self.zoom += -ui.input(|i| i.scroll_delta).y * 0.005;
        self.zoom = self.zoom.clamp(0.1, 50.0);

        let orientation = self.orientation;
        let zoom = self.zoom;
        let origin = self.origin;
        let mesh_center = self.mesh_center;

        let renderer = self.renderer.clone();
        let cb = egui_glow::CallbackFn::new(move |info, _painter| unsafe {
            renderer
                .lock()
                .unwrap()
                .draw(&gl, orientation, origin, zoom, info, mesh_center);
        });
        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

pub struct EntityRenderer {
    grid: glow::Program, // (usize, glow::VertexArray),
    mesh_shader: glow::Program,
    mesh: Option<(usize, glow::VertexArray, glow::Buffer, Vec<TriStrip>)>,

    pub orthographic: bool,
}

impl EntityRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        Self {
            grid: unsafe { Self::create_grid_program(gl).unwrap() },
            mesh_shader: unsafe { Self::create_mesh_program(gl).unwrap() },
            mesh: None,
            orthographic: false,
        }
    }

    /// Returns the center of the model (average of all points)
    unsafe fn load_mesh(&mut self, gl: &glow::Context, mesh: &ProcessedEntityMesh) -> Vec3 {
        let ProcessedEntityMesh {
            vertex_data,
            indices,
            strips,
        } = mesh;

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

        self.mesh = Some((indices.len(), vertex_array, index_buffer, strips.to_vec()));

        (vertex_data
            .iter()
            .map(|v| DVec3::new(v.pos[0] as f64, v.pos[1] as f64, v.pos[2] as f64))
            .sum::<DVec3>()
            / vertex_data.len() as f64)
            .as_vec3()
    }

    unsafe fn create_grid_program(gl: &glow::Context) -> Result<glow::Program, String> {
        let shader_sources = [
            (
                glow::VERTEX_SHADER,
                include_str!("../assets/shaders/grid.vert"),
            ),
            (
                glow::FRAGMENT_SHADER,
                include_str!("../assets/shaders/grid.frag"),
            ),
        ];

        gl_helper::compile_shader(gl, &shader_sources)
    }

    unsafe fn create_mesh_program(gl: &glow::Context) -> Result<glow::Program, String> {
        let shader_sources = [
            (
                glow::VERTEX_SHADER,
                include_str!("../assets/shaders/entity.vert"),
            ),
            (
                glow::FRAGMENT_SHADER,
                include_str!("../assets/shaders/entity.frag"),
            ),
        ];

        gl_helper::compile_shader(gl, &shader_sources)
    }

    pub unsafe fn draw(
        &self,
        gl: &glow::Context,
        orientation: egui::Vec2,
        _origin: Vec3,
        zoom: f32,
        info: egui::PaintCallbackInfo,
        mesh_center: Vec3,
    ) {
        let projection = if self.orthographic {
            glam::Mat4::orthographic_rh_gl(
                info.viewport.aspect_ratio() * -zoom,
                -info.viewport.aspect_ratio() * -zoom,
                1.0 * -zoom,
                -1.0 * -zoom,
                0.0,
                1000.0,
            )
        } else {
            glam::Mat4::perspective_rh_gl(
                90.0_f32.to_radians(),
                info.viewport.aspect_ratio(),
                0.1,
                1000.0,
            )
        };

        let view = glam::Mat4::from_rotation_translation(
            glam::Quat::from_rotation_x(orientation.y) * glam::Quat::from_rotation_z(orientation.x),
            glam::vec3(0.0, 0.0, -5.0 * zoom),
        );

        gl.enable(glow::CULL_FACE);
        gl.cull_face(glow::BACK);
        gl.clear_depth_f32(1.0);
        gl.clear(glow::DEPTH_BUFFER_BIT);
        gl.enable(glow::DEPTH_TEST);

        gl.line_width(1.0);
        gl.use_program(Some(self.grid));
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(self.grid, "u_view").as_ref(),
            false,
            &(projection * view).to_cols_array(),
        );
        gl.draw_arrays(glow::LINES, 0, (25 + 1) * 2 * 2); // 10 lines (+1), 2 points each, 2 sides (horizontal/vertical)

        if let Some((index_count, vertex_array, index_buffer, _strips)) = self.mesh.as_ref() {
            gl.use_program(Some(self.mesh_shader));
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.mesh_shader, "u_view").as_ref(),
                false,
                &(projection * view).to_cols_array(),
            );

            let model = Mat4::from_translation(-mesh_center.zxy());
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.mesh_shader, "u_model")
                    .as_ref(),
                false,
                &model.to_cols_array(),
            );

            gl.bind_vertex_array(Some(*vertex_array));
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*index_buffer));
            gl.draw_elements(glow::TRIANGLES, *index_count as i32, glow::UNSIGNED_INT, 0);
        }
    }
}

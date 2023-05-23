use std::sync::{Arc, Mutex};

use glam::{Vec2, Vec3};

use crate::{
    entities::ProcessedEntityMesh,
    render::{self, entity::EntityRenderer, grid::GridRenderer, RenderUniforms},
};

pub struct EntityFrame {
    pub hashcode: u32,

    pub renderer: Arc<Mutex<EntityRenderer>>,
    orientation: Vec2,
    zoom: f32,

    grid: GridRenderer,
    mesh_center: Vec3,
    pub show_grid: bool,
}

#[derive(Clone)]
pub struct RenderableTexture {
    pub frames: Vec<glow::Texture>,
    pub framerate: usize,
    pub frame_count: usize,
    pub flags: u32,
    pub scroll: Vec2,
}

impl EntityFrame {
    pub fn new(
        gl: &glow::Context,
        hashcode: u32,
        mesh: &ProcessedEntityMesh,
        textures: Vec<RenderableTexture>,
    ) -> Self {
        let mut s = Self {
            hashcode,
            renderer: Arc::new(Mutex::new(EntityRenderer::new(gl, textures))),
            orientation: Vec2::new(-2., -1.),
            zoom: 5.0,
            mesh_center: Vec3::ZERO,
            show_grid: true,
            grid: GridRenderer::new(gl, 30),
        };

        unsafe {
            s.mesh_center = s.renderer.lock().unwrap().load_mesh(gl, mesh);
        }

        s
    }

    fn zoom_factor(zoom_level: f32) -> f32 {
        2.0f32.powf(zoom_level * std::f32::consts::LN_2) - 0.9
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        if let Some(multi_touch) = ui.ctx().multi_touch() {
            self.zoom += -(multi_touch.zoom_delta - 1.0);
        } else {
            self.orientation += Vec2::new(response.drag_delta().x, response.drag_delta().y) * 0.005;

            self.zoom += -ui.input(|i| i.scroll_delta).y * 0.005;
        }

        self.zoom = self.zoom.clamp(0.00, 250.0);

        let orientation = self.orientation;
        let zoom = Self::zoom_factor(self.zoom);
        let mesh_center = self.mesh_center;
        let time = ui.input(|t| t.time);

        let show_grid = self.show_grid;
        let grid = self.grid.clone(); // FIXME: Ugh.

        let renderer = self.renderer.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            render::start_render(painter.gl());

            let renderer_lock = renderer.lock().unwrap();

            let uniforms = RenderUniforms::new(
                renderer_lock.orthographic,
                orientation,
                zoom,
                info.viewport.aspect_ratio(),
            );

            if show_grid {
                grid.draw(&uniforms, painter.gl())
            }

            renderer_lock.draw_both(painter.gl(), &uniforms, mesh_center, time);
        });
        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

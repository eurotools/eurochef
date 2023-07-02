use egui::mutex::{Mutex, RwLock};
use std::sync::Arc;

use eurochef_edb::{versions::Platform, Hashcode};
use glam::{Quat, Vec2, Vec3};
use glow::HasContext;

use crate::{
    entities::ProcessedEntityMesh,
    render::{entity::EntityRenderer, viewer::BaseViewer, RenderStore},
};

pub struct EntityFrame {
    _file: Hashcode,
    render_store: Arc<RwLock<RenderStore>>,
    pub renderers: Vec<Arc<Mutex<EntityRenderer>>>,

    pub viewer: Arc<Mutex<BaseViewer>>,

    mesh_center: Vec3,
    vertex_lighting: bool,
}

#[derive(Clone)]
pub struct RenderableTexture {
    pub frames: Vec<glow::Texture>,
    pub framerate: usize,
    pub frame_count: usize,
    pub flags: u32,
    pub scroll: Vec2,
    pub hashcode: u32,
}

impl EntityFrame {
    pub fn new(
        file: Hashcode,
        render_store: Arc<RwLock<RenderStore>>,
        gl: &glow::Context,
        meshes: &[&ProcessedEntityMesh],
        platform: Platform,
    ) -> Self {
        let mut s = Self {
            _file: file,
            render_store,
            renderers: vec![],
            mesh_center: Vec3::ZERO,
            viewer: Arc::new(Mutex::new(BaseViewer::new(gl))),
            vertex_lighting: true,
        };

        unsafe {
            if meshes.len() > 1 {
                for m in meshes {
                    let r = Arc::new(Mutex::new(EntityRenderer::new(file, platform)));
                    r.lock().load_mesh(gl, m);
                    s.renderers.push(r);
                }
            } else {
                let r = Arc::new(Mutex::new(EntityRenderer::new(file, platform)));
                s.mesh_center = r.lock().load_mesh(gl, meshes[0]);
                s.renderers.push(r);
            }
        }

        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.viewer.lock().show_toolbar(ui);

            if ui
                .checkbox(&mut self.vertex_lighting, "Vertex Lighting")
                .changed()
            {
                // TODO(cohae): Global shaders will make this less painful
                for r in self.renderers.iter() {
                    r.lock().vertex_lighting = self.vertex_lighting;
                }
            }
        });

        egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui));
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        let mesh_center = self.mesh_center;
        let time = ui.input(|t| t.time);

        let viewer = self.viewer.clone();
        viewer.lock().update(ui, &response);

        let render_store = self.render_store.clone();

        let renderers = self.renderers.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            let mut v = viewer.lock();
            v.start_render(painter.gl(), info.viewport.aspect_ratio(), time as f32);
            let render_context = v.render_context();

            for r in &renderers {
                let renderer_lock = r.lock();
                renderer_lock.draw_opaque(
                    painter.gl(),
                    &render_context,
                    -mesh_center,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    time,
                    &render_store.read(),
                );
            }

            painter.gl().depth_mask(false);

            for r in &renderers {
                let renderer_lock = r.lock();
                renderer_lock.draw_transparent(
                    painter.gl(),
                    &render_context,
                    -mesh_center,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    time,
                    &render_store.read(),
                );
            }
        });
        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

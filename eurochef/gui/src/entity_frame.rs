use std::sync::{Arc, Mutex};

use glam::{Vec2, Vec3};
use glow::HasContext;

use crate::{
    entities::ProcessedEntityMesh,
    render::{entity::EntityRenderer, viewer::BaseViewer},
};

pub struct EntityFrame {
    pub textures: Vec<RenderableTexture>,
    pub renderers: Vec<Arc<Mutex<EntityRenderer>>>,

    pub viewer: Arc<Mutex<BaseViewer>>,

    mesh_center: Vec3,
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
        meshes: &[&ProcessedEntityMesh],
        textures: &[RenderableTexture],
    ) -> Self {
        assert!(textures.len() != 0);

        let mut s = Self {
            textures: textures.to_vec(),
            renderers: vec![],
            mesh_center: Vec3::ZERO,
            viewer: Arc::new(Mutex::new(BaseViewer::new(gl))),
        };

        unsafe {
            if meshes.len() > 1 {
                for m in meshes {
                    let r = Arc::new(Mutex::new(EntityRenderer::new(gl)));
                    r.lock().unwrap().load_mesh(gl, m);
                    s.renderers.push(r);
                }
            } else {
                let r = Arc::new(Mutex::new(EntityRenderer::new(gl)));
                s.mesh_center = r.lock().unwrap().load_mesh(gl, meshes[0]);
                s.renderers.push(r);
            }
        }

        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.viewer.lock().unwrap().show_toolbar(ui);
        });

        egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui));
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        let mesh_center = self.mesh_center;
        let time = ui.input(|t| t.time);

        let viewer = self.viewer.clone();
        viewer.lock().unwrap().update(ui, response);

        // TODO(cohae): How do we get out of this situation
        let textures = self.textures.clone(); // FIXME: UUUUGH.

        let renderers = self.renderers.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            viewer
                .lock()
                .unwrap()
                .start_render(painter.gl(), info.viewport.aspect_ratio());

            for r in &renderers {
                let renderer_lock = r.lock().unwrap();
                renderer_lock.draw_opaque(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    -mesh_center,
                    Vec3::ZERO,
                    Vec3::ONE,
                    time,
                    &textures,
                );
            }

            painter.gl().depth_mask(false);

            for r in &renderers {
                let renderer_lock = r.lock().unwrap();
                renderer_lock.draw_transparent(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    -mesh_center,
                    Vec3::ZERO,
                    Vec3::ONE,
                    time,
                    &textures,
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

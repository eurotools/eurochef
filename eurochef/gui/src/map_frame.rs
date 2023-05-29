use std::sync::{Arc, Mutex};

use eurochef_edb::entity::EXGeoEntity;
use glam::Vec3;
use glow::HasContext;

use crate::{
    entities::ProcessedEntityMesh,
    entity_frame::RenderableTexture,
    maps::ProcessedMap,
    render::{entity::EntityRenderer, viewer::BaseViewer},
};

pub struct MapFrame {
    pub textures: Vec<RenderableTexture>,
    pub ref_renderers: Vec<Arc<Mutex<EntityRenderer>>>,
    pub placement_renderers: Vec<(u32, Arc<Mutex<EntityRenderer>>)>,

    pub viewer: Arc<Mutex<BaseViewer>>,
}

impl MapFrame {
    pub fn new(
        gl: &glow::Context,
        meshes: &[&ProcessedEntityMesh],
        textures: &[RenderableTexture],
        entities: &Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
    ) -> Self {
        assert!(textures.len() != 0);

        let mut s = Self {
            textures: textures.to_vec(),
            ref_renderers: vec![],
            placement_renderers: vec![],
            viewer: Arc::new(Mutex::new(BaseViewer::new(gl))),
        };

        unsafe {
            for m in meshes {
                let r = Arc::new(Mutex::new(EntityRenderer::new(gl)));
                r.lock().unwrap().load_mesh(gl, m);
                s.ref_renderers.push(r);
            }

            for (i, _, m) in entities {
                let r = Arc::new(Mutex::new(EntityRenderer::new(gl)));
                r.lock().unwrap().load_mesh(gl, m);
                s.placement_renderers.push((*i, r));
            }
        }

        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, map: &ProcessedMap) {
        self.viewer.lock().unwrap().show_toolbar(ui);

        egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui, map));
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui, map: &ProcessedMap) {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        let time = ui.input(|t| t.time);

        let viewer = self.viewer.clone();
        viewer.lock().unwrap().update(ui, response);

        // TODO(cohae): How do we get out of this situation
        let textures = self.textures.clone(); // FIXME: UUUUGH.
        let map = map.clone(); // FIXME(cohae): ugh.

        let placement_renderers = self.placement_renderers.clone();
        let renderers = self.ref_renderers.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            viewer
                .lock()
                .unwrap()
                .start_render(painter.gl(), info.viewport.aspect_ratio());

            // Render base (ref) entities
            for r in &renderers {
                let renderer_lock = r.lock().unwrap();
                renderer_lock.draw_opaque(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    Vec3::ZERO,
                    Vec3::ZERO,
                    Vec3::ONE,
                    time,
                    &textures,
                );
            }

            for p in &map.placements {
                if let Some((_, r)) = placement_renderers.iter().find(|(i, _)| *i == p.object_ref) {
                    let renderer_lock = r.lock().unwrap();
                    renderer_lock.draw_opaque(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        p.position.into(),
                        p.rotation.into(),
                        p.scale.into(),
                        time,
                        &textures,
                    );
                }
            }

            painter.gl().depth_mask(false);

            for r in &renderers {
                let renderer_lock = r.lock().unwrap();
                renderer_lock.draw_transparent(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    Vec3::ZERO,
                    Vec3::ZERO,
                    Vec3::ONE,
                    time,
                    &textures,
                );
            }

            for p in &map.placements {
                if let Some((_, r)) = placement_renderers.iter().find(|(i, _)| *i == p.object_ref) {
                    let renderer_lock = r.lock().unwrap();
                    renderer_lock.draw_transparent(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        p.position.into(),
                        p.rotation.into(),
                        p.scale.into(),
                        time,
                        &textures,
                    );
                }
            }
        });
        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

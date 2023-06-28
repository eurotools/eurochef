// const COMMAND_COLOR_ENTITY: egui::Color32 = egui::Color32::from_rgb(98, 176, 255);
// const COMMAND_COLOR_EVENT: egui::Color32 = egui::Color32::WHITE;
// const COMMAND_COLOR_UNKNOWN: egui::Color32 = egui::Color32::WHITE;

use std::sync::{Arc, Mutex};

use egui::RichText;
use eurochef_edb::{entity::EXGeoEntity, versions::Platform};
use eurochef_shared::{
    hashcodes::{Hashcode, HashcodeUtils},
    script::{UXGeoScript, UXGeoScriptCommandData},
    IdentifiableResult,
};
use glam::{Quat, Vec3};
use glow::HasContext;
use instant::Instant;
use nohash_hasher::IntMap;

use crate::{
    entities::ProcessedEntityMesh,
    entity_frame::RenderableTexture,
    map_frame::QueuedEntityRender,
    render::{entity::EntityRenderer, viewer::BaseViewer},
};

pub struct ScriptListPanel {
    scripts: IntMap<Hashcode, UXGeoScript>,
    selected_script: Hashcode,
    viewer: Arc<Mutex<BaseViewer>>,
    textures: Vec<RenderableTexture>,
    entities: Vec<(Hashcode, Arc<Mutex<EntityRenderer>>)>,

    current_time: f32,
    is_playing: bool,

    last_frame: Instant,
}

impl ScriptListPanel {
    pub fn new(
        gl: &glow::Context,
        scripts: Vec<UXGeoScript>,
        textures: &[RenderableTexture],
        entities: &Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        platform: Platform,
    ) -> Self {
        let mut s = Self {
            selected_script: scripts.first().map(|s| s.hashcode).unwrap_or(u32::MAX),
            scripts: scripts.into_iter().map(|s| (s.hashcode, s)).collect(),
            viewer: Arc::new(Mutex::new(BaseViewer::new(gl))),
            textures: textures.to_vec(),
            entities: vec![],
            current_time: 0.0,
            is_playing: false,
            last_frame: Instant::now(),
        };

        unsafe {
            for (hashcode, (e, m)) in entities
                .iter()
                .filter(|v| v.data.is_ok())
                .map(|v| (v.hashcode, v.data.as_ref().unwrap()))
            {
                let r = Arc::new(Mutex::new(EntityRenderer::new(&gl, platform)));

                match e {
                    EXGeoEntity::Mesh(_) | EXGeoEntity::Split(_) => {
                        r.lock().unwrap().load_mesh(&gl, m);
                    }
                    _ => {
                        warn!("Creating dud EntityRenderer for EXGeoEntity::0x{:x} with hashcode {:08x}", e.type_code(), hashcode);
                    }
                };

                s.entities.push((hashcode, r));
            }
        }

        s
    }

    fn current_script(&self) -> Option<&UXGeoScript> {
        self.scripts.get(&self.selected_script)
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_source("script_scroll_area")
                    .always_show_scroll(true)
                    .show(ui, |ui| {
                        for hc in self.scripts.keys() {
                            ui.selectable_value(
                                &mut self.selected_script,
                                *hc,
                                format!("{hc:08x}"),
                            );
                        }
                    });
            });

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    self.viewer.lock().unwrap().show_toolbar(ui);
                });

                egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui));

                ui.horizontal(|ui| {
                    if let Some(script) = self.current_script() {
                        ui.strong("Frame:");
                        ui.label(format!(
                            "{}",
                            (self.current_time * script.framerate) as isize
                        ));
                    }
                });

                self.show_controls(ui);
            });
        });

        if self.is_playing {
            self.current_time += delta_time;
        }
        if let Some(script) = self.current_script() {
            if self.current_time > (script.length as f32 / script.framerate) {
                self.current_time = 0.0;
            }
        }
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size() - egui::vec2(0., 64.) - egui::vec2(0., 16.),
            egui::Sense::click_and_drag(),
        );

        let time: f64 = ui.input(|t| t.time);
        let textures = self.textures.clone(); // FIXME: UUUUGH.
        let entities = self.entities.clone();

        let current_frame_commands = if let Some(c) = self.current_script() {
            let current_frame = (self.current_time.floor() * c.framerate) as isize;
            c.commands
                .iter()
                .filter(|c| c.range().contains(&current_frame))
                .cloned()
                .collect()
        } else {
            vec![]
        };

        self.viewer.lock().unwrap().update(ui, &response);
        let viewer = self.viewer.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            let mut v = viewer.lock().unwrap();
            v.start_render(painter.gl(), info.viewport.aspect_ratio(), time as f32);
            let render_context = v.render_context();

            let mut render_queue: Vec<QueuedEntityRender> = vec![];
            for c in &current_frame_commands {
                match c.data {
                    UXGeoScriptCommandData::Entity { hashcode, file } => {
                        if file != u32::MAX {
                            continue;
                        }

                        if let Some((_, renderer)) = entities.get(hashcode.index() as usize) {
                            render_queue.push(QueuedEntityRender {
                                entity: renderer.clone(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                                scale: Vec3::ONE,
                            })
                        }
                    }
                    _ => {}
                }
            }

            for r in render_queue.iter() {
                if let Ok(e) = r.entity.try_lock() {
                    e.draw_opaque(
                        painter.gl(),
                        &render_context,
                        r.position,
                        r.rotation,
                        r.scale,
                        time,
                        &textures,
                    )
                }
            }

            painter.gl().depth_mask(false);

            for r in render_queue.iter() {
                if let Ok(e) = r.entity.try_lock() {
                    e.draw_transparent(
                        painter.gl(),
                        &render_context,
                        r.position,
                        r.rotation,
                        r.scale,
                        time,
                        &textures,
                    )
                }
            }
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }

    fn show_controls(&mut self, ui: &mut egui::Ui) {
        let script = self.current_script();

        ui.horizontal(|ui| {
            ui.style_mut().spacing.button_padding = egui::vec2(6., 4.);

            if ui
                .button(RichText::new(font_awesome::STEP_BACKWARD).size(16.))
                .clicked()
            {}

            if ui
                .button(
                    RichText::new(if self.is_playing {
                        font_awesome::PAUSE
                    } else {
                        font_awesome::PLAY
                    })
                    .size(16.),
                )
                .clicked()
            {
                self.is_playing = !self.is_playing;
            }

            if ui
                .button(RichText::new(font_awesome::STEP_FORWARD).size(16.))
                .clicked()
            {}
        });
    }
}

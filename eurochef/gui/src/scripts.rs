use std::sync::{Arc, Mutex};

use egui::RichText;
use eurochef_edb::{entity::EXGeoEntity, versions::Platform};
use eurochef_shared::{
    hashcodes::{Hashcode, HashcodeUtils},
    maps::format_hashcode,
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
    render::{entity::EntityRenderer, tweeny::ease_in_out_sine, viewer::BaseViewer},
};

pub struct ScriptListPanel {
    scripts: IntMap<Hashcode, UXGeoScript>,
    selected_script: Hashcode,
    viewer: Arc<Mutex<BaseViewer>>,
    textures: Vec<RenderableTexture>,
    entities: Vec<(Hashcode, Arc<Mutex<EntityRenderer>>)>,
    hashcodes: Arc<IntMap<Hashcode, String>>,

    current_time: f32,
    playback_speed: f32,
    is_playing: bool,

    last_frame: Instant,
}

impl ScriptListPanel {
    pub fn new(
        gl: &glow::Context,
        scripts: Vec<UXGeoScript>,
        textures: &[RenderableTexture],
        entities: &Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        hashcodes: Arc<IntMap<Hashcode, String>>,
        platform: Platform,
    ) -> Self {
        let mut s = Self {
            selected_script: scripts.first().map(|s| s.hashcode).unwrap_or(u32::MAX),
            scripts: scripts.into_iter().map(|s| (s.hashcode, s)).collect(),
            viewer: Arc::new(Mutex::new(BaseViewer::new(gl))),
            textures: textures.to_vec(),
            hashcodes,
            entities: vec![],
            current_time: 0.0,
            playback_speed: 1.0,
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

    fn thread_count(&self) -> isize {
        self.current_script()
            .map(|v| {
                v.commands
                    .iter()
                    .map(|c| {
                        if let UXGeoScriptCommandData::Unknown { cmd, .. } = c.data {
                            if cmd == 0x10 || cmd == 0x11 || cmd == 0x12 {
                                0
                            } else {
                                (c.thread as i8) as isize + 1
                            }
                        } else {
                            (c.thread as i8) as isize + 1
                        }
                    })
                    .max()
                    .unwrap_or_default()
            })
            .unwrap_or(0)
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
                        for (i, hc) in self.scripts.keys().enumerate() {
                            if ui
                                .selectable_value(
                                    &mut self.selected_script,
                                    *hc,
                                    format!("{hc:08x} (0x{i:x})"),
                                )
                                .clicked()
                            {
                                self.current_time = 0.0;
                            }
                        }
                    });
            });

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    self.viewer.lock().unwrap().show_toolbar(ui);
                    ui.add(
                        egui::DragValue::new(&mut self.playback_speed)
                            .clamp_range(0.05..=3.0)
                            .speed(0.01),
                    );
                    ui.label("Speed");
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
                ui.add_space(4.0);

                if let Some(script) = self.current_script() {
                    egui::ScrollArea::vertical()
                        .id_source("script_graph_scroll_area")
                        .show(ui, |ui| self.draw_script_graph(script, ui));
                }
            });
        });

        if self.is_playing {
            self.current_time += delta_time * self.playback_speed;
        }
        if let Some(script) = self.current_script() {
            if self.current_time > (script.length as f32 / script.framerate) {
                self.current_time = 0.0;
            }
        }
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            (ui.available_size()
                - egui::vec2(0., 96.)
                - egui::vec2(0., self.thread_count() as f32 * 17.0))
            .clamp(
                egui::vec2(f32::MIN, ui.available_height() / 2.0),
                egui::vec2(f32::MAX, f32::MAX),
            ),
            egui::Sense::click_and_drag(),
        );

        let time: f64 = ui.input(|t| t.time);
        let textures = self.textures.clone(); // FIXME: UUUUGH.
        let entities = self.entities.clone();

        let current_frame_commands = if let Some(c) = self.current_script() {
            let current_frame = (self.current_time * c.framerate).floor() as isize;
            c.commands
                .iter()
                .filter(|c| c.range().contains(&current_frame))
                .cloned()
                .collect()
        } else {
            vec![]
        };

        let mut transforms = vec![];
        for cf in &current_frame_commands {
            let script = self.current_script().unwrap();
            let (pos, rot, scale) = if let Some(c) =
                script.controllers.get(cf.controller_index as usize)
            {
                macro_rules! get_interp_pos {
                    ($v:expr, $default:expr) => {{
                        let mut previous_frame = -1;
                        let mut next_frame = -1;
                        let current_frame = self.current_time * script.framerate;

                        for (i, (start, _)) in $v.iter().enumerate() {
                            if *start >= current_frame {
                                break;
                            }

                            previous_frame = i as isize;
                        }

                        if previous_frame != -1 {
                            next_frame = previous_frame + 1;
                        }

                        if next_frame == -1 || next_frame > $v.len() as isize {
                            next_frame = 0;
                        }

                        let (start, start_value) =
                            if let Some((k, fvalue)) = $v.get(previous_frame as usize) {
                                (*k, *fvalue)
                            } else {
                                (cf.start as f32, $default)
                            };

                        let (end, end_value) =
                            if let Some((k, fvalue)) = $v.get(next_frame as usize) {
                                (*k, *fvalue)
                            } else {
                                (start, start_value)
                            };

                        (start, start_value, end, end_value)
                    }};
                }

                let rot = {
                    let (start, start_rot, end, end_rot) =
                        get_interp_pos!(c.channels.quat_0, Quat::IDENTITY.to_array());

                    let length = end - start;
                    let offset = ((self.current_time * script.framerate) - start) / length;
                    if start == end {
                        Quat::from_array(start_rot)
                    } else {
                        Quat::from_array(start_rot).lerp(Quat::from_array(end_rot), offset)
                    }
                };

                let pos = {
                    let (start, start_pos, end, end_pos) =
                        get_interp_pos!(c.channels.vector_0, Vec3::ZERO.to_array());

                    let length = end - start;
                    let offset = ((self.current_time * script.framerate) - start) / length;
                    if start == end {
                        start_pos.into()
                    } else {
                        Vec3::from(start_pos).lerp(Vec3::from(end_pos), ease_in_out_sine(offset))
                    }
                };

                let scale = {
                    let (start, start_scale, end, end_scale) =
                        get_interp_pos!(c.channels.vector_1, Vec3::ONE.to_array());

                    let length = end - start;
                    let offset = ((self.current_time * script.framerate) - start) / length;
                    if start == end {
                        start_scale.into()
                    } else {
                        Vec3::from(start_scale).lerp(Vec3::from(end_scale), offset)
                    }
                };

                (pos, rot, scale)
            } else {
                (Vec3::ZERO, Quat::IDENTITY, Vec3::ONE)
            };

            transforms.push((pos, rot, scale));
        }

        self.viewer.lock().unwrap().update(ui, &response);
        let viewer = self.viewer.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            let mut v = viewer.lock().unwrap();
            v.start_render(painter.gl(), info.viewport.aspect_ratio(), time as f32);
            let render_context = v.render_context();

            let mut render_queue: Vec<QueuedEntityRender> = vec![];
            for (c, transform) in current_frame_commands.iter().zip(&transforms) {
                match c.data {
                    UXGeoScriptCommandData::Entity { hashcode, file } => {
                        if file != u32::MAX {
                            continue;
                        }

                        if let Some((_, renderer)) = entities.get(hashcode.index() as usize) {
                            render_queue.push(QueuedEntityRender {
                                entity: renderer.clone(),
                                position: transform.0,
                                rotation: transform.1,
                                scale: transform.2,
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
        centerer(ui, |ui| {
            ui.style_mut().spacing.button_padding = egui::vec2(6., 4.);

            if ui
                .button(RichText::new(font_awesome::STEP_BACKWARD).size(16.))
                .clicked()
                || ui.input(|i| i.key_pressed(egui::Key::ArrowLeft))
            {
                if let Some(s) = self.current_script() {
                    let current_frame = (self.current_time * s.framerate) as i32;
                    self.current_time = (current_frame - 1) as f32 / s.framerate;
                }
            }

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
                || ui.input(|i| i.key_pressed(egui::Key::Space))
            {
                self.is_playing = !self.is_playing;
            }

            if ui
                .button(RichText::new(font_awesome::STEP_FORWARD).size(16.))
                .clicked()
                || ui.input(|i| i.key_pressed(egui::Key::ArrowRight))
            {
                if let Some(s) = self.current_script() {
                    let current_frame = (self.current_time * s.framerate) as i32;
                    self.current_time = (current_frame + 1) as f32 / s.framerate;
                }
            }
        });
    }

    const COMMAND_COLOR_ENTITY: egui::Color32 = egui::Color32::from_rgb(98, 176, 255);
    const COMMAND_COLOR_PARTICLE: egui::Color32 = egui::Color32::from_rgb(168, 235, 247);
    const COMMAND_COLOR_SUBSCRIPT: egui::Color32 = egui::Color32::from_rgb(238, 145, 234);
    const COMMAND_COLOR_SOUND: egui::Color32 = egui::Color32::from_rgb(255, 188, 255);
    const COMMAND_COLOR_EVENT: egui::Color32 = egui::Color32::WHITE;
    const COMMAND_COLOR_UNKNOWN: egui::Color32 = egui::Color32::WHITE;

    fn draw_script_graph(&self, script: &UXGeoScript, ui: &mut egui::Ui) {
        let num_threads = script
            .commands
            .iter()
            .map(|v| v.thread as i8 + 1)
            .max()
            .unwrap();

        let current_frame = self.current_time * script.framerate;
        let width = ui.available_width();
        let single_frame_width = width / script.length as f32;

        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(width, num_threads as f32 * 17.0),
            egui::Sense::click(),
        );

        for c in &script.commands {
            let (color, label, file_hash) = match &c.data {
                UXGeoScriptCommandData::Entity { hashcode, file } => (
                    Self::COMMAND_COLOR_ENTITY,
                    format!("Entity {}", format_hashcode(&self.hashcodes, *hashcode)),
                    *file,
                ),
                UXGeoScriptCommandData::SubScript { hashcode, file } => (
                    Self::COMMAND_COLOR_SUBSCRIPT,
                    format!("Sub-Script {}", format_hashcode(&self.hashcodes, *hashcode)),
                    *file,
                ),
                UXGeoScriptCommandData::Sound { hashcode } => (
                    Self::COMMAND_COLOR_SOUND,
                    format!("Sound {}", format_hashcode(&self.hashcodes, *hashcode)),
                    u32::MAX,
                ),
                UXGeoScriptCommandData::Particle { hashcode, file } => (
                    Self::COMMAND_COLOR_PARTICLE,
                    format!("Particle {}", format_hashcode(&self.hashcodes, *hashcode)),
                    *file,
                ),
                UXGeoScriptCommandData::EventType { event_type } => (
                    Self::COMMAND_COLOR_EVENT,
                    format!("Event {}", format_hashcode(&self.hashcodes, *event_type)),
                    u32::MAX,
                ),
                UXGeoScriptCommandData::Unknown { cmd, .. } => {
                    if *cmd == 0x10 || *cmd == 0x11 || *cmd == 0x12 {
                        continue;
                    }

                    (
                        Self::COMMAND_COLOR_UNKNOWN,
                        format!("Unknown 0x{cmd:x}"),
                        u32::MAX,
                    )
                }
            };

            let start = c.start.clamp(0, i16::MAX);
            let cmd_response = ui.allocate_rect(
                egui::Rect::from_min_size(
                    rect.min
                        + egui::vec2(start as f32 * single_frame_width, c.thread as f32 * 19.0),
                    egui::vec2(c.length as f32 * single_frame_width, 18.0),
                ),
                egui::Sense::hover(),
            );

            cmd_response.on_hover_text_at_pointer(format!(
                "{}{}\nStart: {}\nLength: {}\nController: {}",
                label,
                if file_hash != u32::MAX {
                    format!(" ({})", format_hashcode(&self.hashcodes, file_hash))
                } else {
                    String::new()
                },
                c.start,
                c.length,
                c.controller_index
            ));

            let cmd_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(start as f32 * single_frame_width, c.thread as f32 * 19.0),
                egui::vec2(c.length as f32 * single_frame_width, 18.0),
            );
            let graph_paint_clipped = ui.painter_at(cmd_rect);

            graph_paint_clipped.rect_filled(cmd_rect, egui::Rounding::same(4.0), color);

            if let Some(controller) = script.controllers.get(c.controller_index as usize) {
                let mut keyframes: Vec<f32> = controller
                    .channels
                    .vector_0
                    .iter()
                    .map(|(f, _)| *f)
                    .chain(controller.channels.quat_0.iter().map(|(f, _)| *f))
                    .chain(controller.channels.vector_1.iter().map(|(f, _)| *f))
                    .collect();

                keyframes.sort_by(|a, b| a.partial_cmp(b).unwrap());
                keyframes.dedup();

                for k in keyframes {
                    graph_paint_clipped.text(
                        rect.min
                            + egui::vec2(k * single_frame_width, c.thread as f32 * 19.0 + 18.5),
                        egui::Align2::CENTER_BOTTOM,
                        "🔺",
                        egui::FontId::proportional(6.0),
                        egui::Color32::BLACK,
                    );
                }
            }

            graph_paint_clipped.text(
                rect.min
                    + egui::vec2(
                        4.0 + start as f32 * single_frame_width,
                        c.thread as f32 * 19.0 + 9.0,
                    ),
                egui::Align2::LEFT_CENTER,
                format!("{} - {}", c.start, label),
                egui::FontId::proportional(12.0),
                egui::Color32::BLACK,
            );
        }

        // Render playhead
        ui.painter_at(rect).vline(
            rect.min.x + current_frame * single_frame_width,
            rect.min.y..=(rect.min.y + num_threads as f32 * 19.0),
            egui::Stroke::new(1.0, egui::Color32::RED),
        );
    }
}

// Helper function to center arbitrary widgets. It works by measuring the width of the widgets after rendering, and
// then using that offset on the next frame.
fn centerer(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let id = ui.id().with("_centerer");
        let last_width: Option<f32> = ui.memory_mut(|mem| mem.data.get_temp(id));
        if let Some(last_width) = last_width {
            ui.add_space((ui.available_width() - last_width) / 2.0);
        }
        let res = ui
            .scope(|ui| {
                add_contents(ui);
            })
            .response;
        let width = res.rect.width();
        ui.memory_mut(|mem| mem.data.insert_temp(id, width));

        // Repaint if width changed
        match last_width {
            None => ui.ctx().request_repaint(),
            Some(last_width) if last_width != width => ui.ctx().request_repaint(),
            Some(_) => {}
        }
    });
}

use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use egui::{emath, Pos2, Rect, Vec2};
use eurochef_edb::{
    entity::{EXGeoBaseEntity, EXGeoEntity},
    versions::Platform,
};
use eurochef_shared::IdentifiableResult;
use glam::{Quat, Vec3};
use glow::HasContext;

use crate::{
    entities::ProcessedEntityMesh,
    entity_frame::RenderableTexture,
    maps::ProcessedMap,
    render::{
        billboard::BillboardRenderer,
        blend::{set_blending_mode, BlendMode},
        camera::Camera3D,
        entity::EntityRenderer,
        gl_helper,
        pickbuffer::{PickBuffer, PickBufferType},
        trigger::{LinkLineRenderer, SelectCubeRenderer},
        viewer::{BaseViewer, CameraType},
    },
};

pub struct MapFrame {
    gl: Arc<glow::Context>,
    pub textures: Vec<RenderableTexture>,
    pub ref_renderers: Vec<(u32, Arc<Mutex<EntityRenderer>>)>,
    pub placement_renderers: Vec<(u32, EXGeoBaseEntity, Arc<Mutex<EntityRenderer>>)>,
    billboard_renderer: Arc<BillboardRenderer>,
    trigger_texture: glow::Texture,
    link_renderer: Arc<LinkLineRenderer>,
    selected_trigger: Option<usize>,
    select_renderer: Arc<SelectCubeRenderer>,

    pub viewer: Arc<Mutex<BaseViewer>>,
    sky_ent: String,

    /// Used to prevent keybinds being triggered while a textfield is focused
    textfield_focused: bool,

    vertex_lighting: bool,
    show_triggers: bool,
    // ray_debug: Option<RayDebug>,
    pickbuffer: PickBuffer,

    selected_map: usize,
    trigger_scale: f32,
}

const DEFAULT_ICON_DATA: &[u8] = include_bytes!("../../../assets/icons/triggers/default.png");

impl MapFrame {
    pub fn new(
        gl: Arc<glow::Context>,
        meshes: &[(u32, &ProcessedEntityMesh)],
        textures: &[RenderableTexture],
        entities: &Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        platform: Platform,
    ) -> Self {
        assert!(textures.len() != 0);

        let (default_icon_data, default_icon_info) = {
            let mut cursor = Cursor::new(DEFAULT_ICON_DATA);
            let mut decoder = png::Decoder::new(&mut cursor);
            decoder.set_transformations(png::Transformations::normalize_to_color8());
            let mut reader = decoder.read_info().unwrap();
            let mut img_data = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut img_data).unwrap();
            (img_data[..info.buffer_size()].to_vec(), info)
        };

        let mut s = Self {
            textures: textures.to_vec(),
            ref_renderers: vec![],
            placement_renderers: vec![],
            viewer: Arc::new(Mutex::new(BaseViewer::new(&gl))),
            sky_ent: String::new(),
            textfield_focused: false,
            vertex_lighting: true,
            show_triggers: true,
            billboard_renderer: Arc::new(BillboardRenderer::new(&gl).unwrap()),
            link_renderer: Arc::new(LinkLineRenderer::new(&gl).unwrap()),
            select_renderer: Arc::new(SelectCubeRenderer::new(&gl).unwrap()),
            trigger_texture: unsafe {
                gl_helper::load_texture(
                    &gl,
                    default_icon_info.width as i32,
                    default_icon_info.height as i32,
                    &default_icon_data,
                    glow::RGBA,
                    0,
                )
            },
            selected_trigger: None,
            pickbuffer: PickBuffer::new(&gl),
            gl: gl.clone(),
            selected_map: 0,
            trigger_scale: 0.25,
        };

        unsafe {
            for (i, m) in meshes {
                let r = Arc::new(Mutex::new(EntityRenderer::new(&gl, platform)));
                r.lock().unwrap().load_mesh(&gl, m);
                s.ref_renderers.push((*i, r));
            }

            for (i, (e, m)) in entities
                .iter()
                .filter(|v| v.data.is_ok())
                .map(|v| (v.hashcode, v.data.as_ref().unwrap()))
            {
                // Only allow split/normal meshes
                match e {
                    EXGeoEntity::Mesh(_) => {}
                    EXGeoEntity::Split(_) => {}
                    _ => continue,
                };

                let r = Arc::new(Mutex::new(EntityRenderer::new(&gl, platform)));
                r.lock().unwrap().load_mesh(&gl, m);

                let base = e.base().unwrap().clone();
                s.placement_renderers.push((i, base, r));
            }
        }

        s.placement_renderers
            .sort_by(|(_, e, _), (_, e2, _)| e.sort_value.cmp(&e2.sort_value));

        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, maps: &[ProcessedMap]) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Map")
                .selected_text({
                    let map = &maps[self.selected_map];
                    format!("{:x} ({} zones)", map.hashcode, map.mapzone_entities.len())
                })
                .show_ui(ui, |ui| {
                    for (i, m) in maps.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.selected_map,
                            i,
                            format!("{:x} ({} zones)", m.hashcode, m.mapzone_entities.len()),
                        );
                    }
                });

            self.viewer.lock().unwrap().show_toolbar(ui);

            ui.label("  |  ");

            let response = egui::TextEdit::singleline(&mut self.sky_ent)
                .desired_width(76.0)
                .show(ui)
                .response;

            self.textfield_focused = response.has_focus();

            if let Ok(hashcode) = u32::from_str_radix(&self.sky_ent, 16) {
                if !self
                    .placement_renderers
                    .iter()
                    .find(|(hc, _, _)| *hc == hashcode)
                    .is_some()
                {
                    ui.strong(font_awesome::EXCLAMATION_TRIANGLE.to_string())
                        .on_hover_ui(|ui| {
                            ui.label("Entity was not found");
                        });
                }
            } else {
                ui.strong(font_awesome::EXCLAMATION_TRIANGLE.to_string())
                    .on_hover_ui(|ui| {
                        ui.label("String is not formatted as a valid hashcode");
                    });
            }
            ui.label("Sky ent");

            if ui
                .checkbox(&mut self.vertex_lighting, "Vertex Lighting")
                .changed()
            {
                // TODO(cohae): Global shaders will make this less painful
                for r in self
                    .ref_renderers
                    .iter()
                    .map(|(_, v)| v)
                    .chain(self.placement_renderers.iter().map(|r| &r.2))
                {
                    r.lock().unwrap().vertex_lighting = self.vertex_lighting;
                }
            }

            ui.checkbox(&mut self.show_triggers, "Show Triggers");

            ui.add(
                egui::DragValue::new(&mut self.trigger_scale)
                    .clamp_range(0.25..=2.0)
                    .max_decimals(2)
                    .speed(0.05),
            );
            ui.label("Trigger scale");
        });
        let map = &maps[self.selected_map];

        egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui, map));

        ui.horizontal(|ui| {
            self.viewer.lock().unwrap().show_statusbar(ui);
            if let Some(trig_id) = self.selected_trigger {
                ui.strong("Selected trigger:");
                ui.label(format!("{}", trig_id));
            }
        });
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui, map: &ProcessedMap) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size() - egui::vec2(0., 16.),
            egui::Sense::click_and_drag(),
        );

        if response.clicked() && self.show_triggers {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.render_pickbuffer(rect.size(), map);
                let to_screen = emath::RectTransform::from_to(
                    Rect::from_min_size(Pos2::ZERO, rect.size()),
                    response.rect,
                );
                let from_screen = to_screen.inverse();

                let viewport_pos = from_screen * pointer_pos;
                let viewport_pos = egui::pos2(viewport_pos.x, rect.height() - viewport_pos.y);
                let mut pixel = [0u8; 4];
                unsafe {
                    self.gl
                        .bind_framebuffer(glow::FRAMEBUFFER, self.pickbuffer.framebuffer);
                    self.gl.read_pixels(
                        viewport_pos.x as i32,
                        viewport_pos.y as i32,
                        1,
                        1,
                        glow::RGB,
                        glow::UNSIGNED_BYTE,
                        glow::PixelPackData::Slice(&mut pixel),
                    );
                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                }

                let id = u32::from_le_bytes(pixel);
                let ty = (id >> 20) & 0x0f;
                let id = id & 0x0fffff;

                if ty == 1 {
                    self.selected_trigger = Some(id as usize);
                } else {
                    self.selected_trigger = None;
                }
            }
        }

        let time: f64 = ui.input(|t| t.time);

        let viewer = self.viewer.clone();
        let (camera_pos, camera_rot) = {
            let mut v = viewer.lock().unwrap();
            if !self.textfield_focused {
                v.update(ui, &response);
            }

            let camera: &mut dyn Camera3D = match v.selected_camera {
                CameraType::Fly => &mut v.camera_fly,
                CameraType::Orbit => &mut v.camera_orbit,
            };

            (camera.position(), camera.rotation())
        };
        // TODO(cohae): Why is this necessary?
        let camera_pos = Vec3::new(-camera_pos.x, camera_pos.y, camera_pos.z);

        // TODO(cohae): How do we get out of this situation
        let textures = self.textures.clone(); // FIXME: UUUUGH.
        let map = map.clone(); // FIXME(cohae): ugh.
        let sky_ent = u32::from_str_radix(&self.sky_ent, 16).unwrap_or(u32::MAX);
        let trigger_texture = self.trigger_texture.clone();
        let billboard_renderer = self.billboard_renderer.clone();
        let link_renderer = self.link_renderer.clone();
        let selected_trigger = self.selected_trigger;
        let select_renderer = self.select_renderer.clone();
        let show_triggers = self.show_triggers;
        let trigger_scale = self.trigger_scale;

        let placement_renderers = self.placement_renderers.clone();
        let renderers = self.ref_renderers.clone();
        let cb = egui_glow::CallbackFn::new(move |info, painter| unsafe {
            viewer.lock().unwrap().start_render(
                painter.gl(),
                info.viewport.aspect_ratio(),
                time as f32,
            );

            if let Some((_, _, sky_renderer)) =
                placement_renderers.iter().find(|(hc, _, _)| *hc == sky_ent)
            {
                painter.gl().depth_mask(false);

                sky_renderer.lock().unwrap().draw_both(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    camera_pos,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    time,
                    &textures,
                );

                painter.gl().depth_mask(true);
            }

            // Render base (ref) entities
            for (_, r) in renderers.iter().filter(|(i, _)| *i == map.hashcode) {
                let renderer_lock = r.lock().unwrap();
                renderer_lock.draw_opaque(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    time,
                    &textures,
                );
            }

            for p in &map.placements {
                if let Some((_, base, r)) = placement_renderers
                    .iter()
                    .find(|(i, _, _)| *i == p.object_ref)
                {
                    let mut rotation: Quat = Quat::from_euler(
                        glam::EulerRot::ZXY,
                        p.rotation[2],
                        p.rotation[0],
                        p.rotation[1],
                    );
                    let position: Vec3 = p.position.into();
                    if (base.flags & 0x4) != 0 {
                        rotation = -camera_rot;
                    }

                    let renderer_lock = r.lock().unwrap();
                    renderer_lock.draw_opaque(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        position,
                        rotation,
                        p.scale.into(),
                        time,
                        &textures,
                    );
                }
            }

            painter.gl().depth_mask(false);

            for (_, r) in renderers.iter().filter(|(i, _)| *i == map.hashcode) {
                let renderer_lock = r.lock().unwrap();
                renderer_lock.draw_transparent(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    time,
                    &textures,
                );
            }

            for p in &map.placements {
                if let Some((_, base, r)) = placement_renderers
                    .iter()
                    .find(|(i, _, _)| *i == p.object_ref)
                {
                    let mut rotation: Quat = Quat::from_euler(
                        glam::EulerRot::ZXY,
                        p.rotation[2],
                        p.rotation[0],
                        p.rotation[1],
                    );
                    let position: Vec3 = p.position.into();
                    // TODO(cohae): Shouldn't this be part of the entity renderer?
                    if (base.flags & 0x4) != 0 {
                        rotation = -camera_rot;
                    }

                    let renderer_lock = r.lock().unwrap();
                    renderer_lock.draw_transparent(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        position,
                        rotation,
                        p.scale.into(),
                        time,
                        &textures,
                    );
                }
            }

            if show_triggers {
                painter.gl().depth_mask(true);
                if let Some(Some(trig)) = selected_trigger.map(|v| map.triggers.get(v)) {
                    for l in &trig.links {
                        if *l != -1 {
                            if *l >= map.triggers.len() as i32 {
                                warn!("Trigger doesnt exist! ({l})");
                                continue;
                            }

                            let end = map.triggers[*l as usize].position;
                            link_renderer.render(
                                painter.gl(),
                                &viewer.lock().unwrap().uniforms,
                                trig.position,
                                end,
                                Vec3::new(0.913, 0.547, 0.125),
                                trigger_scale,
                            );
                        }
                    }

                    for l in &trig.incoming_links {
                        if *l != -1 {
                            if *l >= map.triggers.len() as i32 {
                                warn!("Trigger doesnt exist! ({l})");
                                continue;
                            }

                            let end = map.triggers[*l as usize].position;
                            link_renderer.render(
                                painter.gl(),
                                &viewer.lock().unwrap().uniforms,
                                end,
                                trig.position,
                                Vec3::new(0.169, 0.554, 0.953),
                                trigger_scale,
                            );
                        }
                    }

                    select_renderer.render(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        trig.position,
                        // TODO(cohae): This scaling is too small for spyro
                        trigger_scale,
                    );
                }

                for t in map.triggers.iter() {
                    billboard_renderer.render(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        trigger_texture,
                        t.position,
                        // TODO(cohae): This scaling is too small for spyro
                        trigger_scale,
                    );
                }
                set_blending_mode(painter.gl(), BlendMode::None);
            }
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }

    fn render_pickbuffer(&mut self, res: Vec2, map: &ProcessedMap) {
        self.pickbuffer
            .init_draw(&self.gl, glam::ivec2(res.x as i32, res.y as i32));
        for (i, t) in map.triggers.iter().enumerate() {
            self.billboard_renderer.render_pickbuffer(
                &self.gl,
                &self.viewer.lock().unwrap().uniforms,
                t.position,
                // TODO(cohae): This scaling is too small for spyro
                self.trigger_scale,
                (PickBufferType::Trigger, i as u32),
                &self.pickbuffer,
            );
        }
    }
}

use std::{
    fs::File,
    io::Cursor,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use egui::{emath, Pos2, Rect, Vec2};
use eurochef_edb::{
    entity::{EXGeoBaseEntity, EXGeoEntity},
    versions::Platform,
};
use eurochef_shared::{
    maps::{TrigDataType, TriggerInformation},
    IdentifiableResult,
};
use fxhash::FxHashMap;
use glam::{Quat, Vec3};
use glow::HasContext;
use nohash_hasher::IntMap;

use crate::{
    entities::ProcessedEntityMesh,
    entity_frame::RenderableTexture,
    maps::{ProcessedMap, ProcessedTrigger},
    render::{
        billboard::BillboardRenderer,
        blend::{set_blending_mode, BlendMode},
        entity::EntityRenderer,
        gl_helper,
        pickbuffer::{PickBuffer, PickBufferType},
        trigger::{CollisionCubeRenderer, LinkLineRenderer, SelectCubeRenderer},
        tweeny::{self, Tweeny3D},
        viewer::BaseViewer,
    },
};

pub struct MapFrame {
    gl: Arc<glow::Context>,
    pub textures: Vec<RenderableTexture>,
    pub ref_renderers: Vec<(u32, Arc<Mutex<EntityRenderer>>)>,
    pub placement_renderers: Vec<(u32, EXGeoBaseEntity, Arc<Mutex<EntityRenderer>>)>,
    billboard_renderer: Arc<BillboardRenderer>,
    collision_renderer: Arc<CollisionCubeRenderer>,
    default_trigger_icon: glow::Texture,
    link_renderer: Arc<LinkLineRenderer>,
    selected_trigger: Option<usize>,
    selected_link: Option<i32>,
    select_renderer: Arc<SelectCubeRenderer>,

    pub viewer: Arc<Mutex<BaseViewer>>,
    sky_ent: String,

    /// Used to prevent keybinds being triggered while a textfield is focused
    textfield_focused: bool,

    vertex_lighting: bool,
    show_triggers: bool,
    pickbuffer: PickBuffer,

    selected_map: usize,
    trigger_scale: f32,
    trigger_focus_tween: Option<Tweeny3D>,

    trigger_info: Arc<TriggerInformation>,
    selected_triginfo_path: String,
    available_triginfo_paths: Vec<String>,

    hashcodes: Arc<IntMap<u32, String>>,
    trigger_icons: Arc<FxHashMap<String, glow::Texture>>,
}

const DEFAULT_ICON_DATA: &[u8] = include_bytes!("../../../assets/icons/triggers/default.png");

fn load_png_frame(data: &[u8]) -> (Vec<u8>, png::OutputInfo) {
    let mut cursor = Cursor::new(data);
    let mut decoder = png::Decoder::new(&mut cursor);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut img_data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut img_data).unwrap();
    (img_data[..info.buffer_size()].to_vec(), info)
}

impl MapFrame {
    pub fn new(
        gl: Arc<glow::Context>,
        meshes: &[(u32, &ProcessedEntityMesh)],
        textures: &[RenderableTexture],
        entities: &Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        platform: Platform,
        hashcodes: Arc<IntMap<u32, String>>,
    ) -> Self {
        assert!(textures.len() != 0);

        let (default_icon_data, default_icon_info) = load_png_frame(DEFAULT_ICON_DATA);

        let mut available_triginfo_paths = vec![];
        let exe_path = std::env::current_exe().unwrap();
        let exe_dir = exe_path.parent().unwrap();
        if let Ok(d) = exe_dir.join("assets").read_dir() {
            available_triginfo_paths = d
                .filter(|d| {
                    d.as_ref().unwrap().file_type().unwrap().is_file()
                        && d.as_ref()
                            .unwrap()
                            .file_name()
                            .to_os_string()
                            .to_string_lossy()
                            .to_lowercase()
                            .ends_with(".yml")
                })
                .map(|d| {
                    d.as_ref()
                        .unwrap()
                        .file_name()
                        .as_os_str()
                        .to_string_lossy()
                        .to_string()
                })
                .collect();
        }

        let mut trigger_icons = FxHashMap::default();
        if let Ok(d) = exe_dir.join("./assets/icons/triggers").read_dir() {
            for p in d
                .filter(|d| d.as_ref().unwrap().file_type().unwrap().is_file())
                .map(|d| {
                    d.as_ref()
                        .unwrap()
                        .file_name()
                        .as_os_str()
                        .to_string_lossy()
                        .to_string()
                })
                .filter(|d| d.to_lowercase().ends_with(".png"))
            {
                let mut file =
                    File::open(exe_dir.join("./assets/icons/triggers").join(&p)).unwrap();
                let mut decoder = png::Decoder::new(&mut file);
                decoder.set_transformations(png::Transformations::normalize_to_color8());
                let mut reader = decoder.read_info().unwrap();
                let mut img_data = vec![0; reader.output_buffer_size()];
                let info = reader.next_frame(&mut img_data).unwrap();

                let name = p.trim_end_matches(".png");
                trigger_icons.insert(name.to_lowercase(), unsafe {
                    gl_helper::load_texture(
                        &gl,
                        info.width as i32,
                        info.height as i32,
                        &img_data[..info.buffer_size()],
                        glow::RGBA,
                        0,
                    )
                });
            }
        }

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
            default_trigger_icon: unsafe {
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
            collision_renderer: Arc::new(CollisionCubeRenderer::new(&gl).unwrap()),
            gl: gl.clone(),
            selected_map: 0,
            trigger_scale: 0.5,
            trigger_focus_tween: None,
            selected_link: None,
            trigger_info: Default::default(),
            selected_triginfo_path: String::new(),
            available_triginfo_paths,
            hashcodes,
            trigger_icons: Arc::new(trigger_icons),
        };

        unsafe {
            for (i, m) in meshes {
                let r = Arc::new(Mutex::new(EntityRenderer::new(&gl, platform)));
                r.lock().unwrap().load_mesh(&gl, m);
                s.ref_renderers.push((*i, r));
            }

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

                let base = e.base().unwrap().clone();
                s.placement_renderers.push((hashcode, base, r));
            }
        }

        // s.placement_renderers
        //     .sort_by(|(_, e, _), (_, e2, _)| e.sort_value.cmp(&e2.sort_value));

        s
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        context: &egui::Context,
        maps: &[ProcessedMap],
    ) -> anyhow::Result<()> {
        self.selected_link = None;
        ui.horizontal(|ui| -> anyhow::Result<()> {
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
                    .clamp_range(0.1..=2.0)
                    .max_decimals(2)
                    .speed(0.05),
            );
            ui.label("Trigger scale");

            let trig_resp = egui::ComboBox::from_label("Triggers")
                .selected_text(&self.selected_triginfo_path)
                .width(164.0)
                .show_ui(ui, |ui| {
                    let mut resp = ui.selectable_value(
                        &mut self.selected_triginfo_path,
                        String::new(),
                        "None",
                    );
                    for p in &self.available_triginfo_paths {
                        resp = resp.union(ui.selectable_value(
                            &mut self.selected_triginfo_path,
                            p.to_string(),
                            p,
                        ));
                    }
                    resp
                });

            let trig_reload_resp = ui.button("\u{f2f1}");

            if trig_resp.inner.map(|i| i.changed()).unwrap_or_default()
                || trig_reload_resp.clicked()
            {
                if self.selected_triginfo_path.is_empty() {
                    self.trigger_info = Default::default();
                } else {
                    let exe_path = std::env::current_exe().unwrap();
                    let exe_dir = exe_path.parent().unwrap();
                    let v = std::fs::read_to_string(
                        exe_dir.join(&format!("./assets/{}", self.selected_triginfo_path)),
                    )
                    .unwrap();
                    self.trigger_info = serde_yaml::from_str(&v)
                        .context("Failed to load trigger definition file")?;
                    self.trigger_scale = self.trigger_info.icon_scale;
                }
            }

            Ok(())
        })
        .inner?;
        let map = &maps[self.selected_map];

        egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui, context, map));

        ui.horizontal(|ui| {
            self.viewer.lock().unwrap().show_statusbar(ui);
            if let Some(trig_id) = self.selected_trigger {
                ui.strong("Selected trigger:");
                ui.label(format!("{}", trig_id));
            }
        });

        Ok(())
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui, context: &egui::Context, map: &ProcessedMap) {
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

        self.draw_trigger_inspector(context, &map);

        let time: f64 = ui.input(|t| t.time);

        let viewer = self.viewer.clone();
        let (camera_pos, camera_rot) = {
            let mut v = viewer.lock().unwrap();
            if !self.textfield_focused {
                v.update(ui, &response);
            }

            let camera = v.camera_mut();
            let (cp, cr) = (camera.position(), camera.rotation());

            if let Some(tween) = &mut self.trigger_focus_tween {
                if tween.is_finished() {
                    self.trigger_focus_tween = None;
                } else {
                    let p = tween.update();
                    v.focus_on_point(p, self.trigger_scale);
                }
            }

            (cp, cr)
        };

        // TODO(cohae): How do we get out of this situation
        let textures = self.textures.clone(); // FIXME: UUUUGH.
        let map = map.clone(); // FIXME(cohae): ugh.
        let sky_ent = u32::from_str_radix(&self.sky_ent, 16).unwrap_or(u32::MAX);
        let default_trigger_icon = self.default_trigger_icon.clone();
        let billboard_renderer = self.billboard_renderer.clone();
        let link_renderer = self.link_renderer.clone();
        let selected_trigger = self.selected_trigger;
        let select_renderer = self.select_renderer.clone();
        let show_triggers = self.show_triggers;
        let trigger_scale = self.trigger_scale;
        let hovered_link = self.selected_link;
        let trigger_info = self.trigger_info.clone();
        let trigger_icons = self.trigger_icons.clone();

        let collision_renderer = self.collision_renderer.clone();
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

            for t in map.triggers.iter() {
                if let Some(Some(v)) = t.engine_data.get(0) {
                    // Render if it's a local entity
                    if (*v & 0xff000000) == 0x82000000 {
                        let renderer_lock = &placement_renderers[(*v & 0x0000ffff) as usize]
                            .2
                            .lock()
                            .unwrap();

                        let rotation: Quat = Quat::from_euler(
                            glam::EulerRot::ZXY,
                            t.rotation[2],
                            t.rotation[0],
                            t.rotation[1],
                        );

                        renderer_lock.draw_opaque(
                            painter.gl(),
                            &viewer.lock().unwrap().uniforms,
                            t.position,
                            rotation,
                            t.scale,
                            time,
                            &textures,
                        );
                    }
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

            for t in map.triggers.iter() {
                if let Some(Some(v)) = t.engine_data.get(0) {
                    // Render if it's a local entity
                    if (*v & 0xff000000) == 0x82000000 {
                        let renderer_lock = &placement_renderers[(*v & 0x0000ffff) as usize]
                            .2
                            .lock()
                            .unwrap();

                        let rotation: Quat = Quat::from_euler(
                            glam::EulerRot::ZXY,
                            t.rotation[2],
                            t.rotation[0],
                            t.rotation[1],
                        );

                        renderer_lock.draw_transparent(
                            painter.gl(),
                            &viewer.lock().unwrap().uniforms,
                            t.position,
                            rotation,
                            t.scale,
                            time,
                            &textures,
                        );
                    }
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
                                if hovered_link.map(|v| v == *l).unwrap_or_default() {
                                    Vec3::ONE
                                } else {
                                    Vec3::new(0.913, 0.547, 0.125)
                                },
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
                                if hovered_link.map(|v| v == *l).unwrap_or_default() {
                                    Vec3::ONE
                                } else {
                                    Vec3::new(0.169, 0.554, 0.953)
                                },
                                trigger_scale,
                            );
                        }
                    }

                    select_renderer.render(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        trig.position,
                        Quat::from_euler(
                            glam::EulerRot::ZXY,
                            trig.rotation.z,
                            trig.rotation.x,
                            trig.rotation.y,
                        ),
                        trigger_scale,
                    );
                }

                for t in map.triggers.iter() {
                    let trigger_texture_path = trigger_info
                        .triggers
                        .get(&t.ttype)
                        .and_then(|m| m.icon.as_ref().map(|v| v.to_lowercase()));

                    let trigger_texture = *trigger_texture_path
                        .and_then(|p| trigger_icons.get(&p))
                        .unwrap_or(&default_trigger_icon);

                    billboard_renderer.render(
                        painter.gl(),
                        &viewer.lock().unwrap().uniforms,
                        trigger_texture,
                        t.position,
                        trigger_scale,
                    );
                }
                set_blending_mode(painter.gl(), BlendMode::None);

                // Trigger collisions
                set_blending_mode(painter.gl(), BlendMode::Blend);
                // for t in map.triggers.iter() {
                if let Some(t) = selected_trigger.and_then(|t| map.triggers.get(t)) {
                    if let Some(Some(coll)) = t
                        .engine_data
                        .get(3)
                        .map(|c| c.map(|c| map.trigger_collisions.get(c as usize)))
                        .flatten()
                    {
                        if coll.dtype == 0 || coll.dtype == 3 {
                            collision_renderer.render(
                                painter.gl(),
                                &viewer.lock().unwrap().uniforms,
                                t.position + Vec3::from(coll.position),
                                Quat::from_euler(
                                    glam::EulerRot::ZXY,
                                    t.rotation.z,
                                    t.rotation.x,
                                    t.rotation.y,
                                ),
                                coll,
                            );
                        }
                    }
                }
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
                self.trigger_scale,
                (PickBufferType::Trigger, i as u32),
                &self.pickbuffer,
            );
        }
    }

    fn draw_trigger_inspector(&mut self, ctx: &egui::Context, map: &ProcessedMap) {
        let screen_space = ctx.screen_rect();
        egui::Window::new("Inspector")
            .scroll2([false, true])
            .show(ctx, |ui| {
                if self.selected_trigger.is_none() || !self.show_triggers {
                    ui.heading("No object selected");
                    return;
                }

                macro_rules! readonly_input {
                    ($ui:expr, $string:expr) => {
                        let mut tmp = $string;
                        $ui.add_enabled(false, egui::TextEdit::singleline(&mut tmp));
                    };
                    ($ui:expr, $label:expr, $string:expr) => {
                        // $ui.horizontal(|ui| {
                        $ui.label($label);
                        let mut tmp = $string;
                        $ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut tmp), // .desired_width(f32::INFINITY),
                        );
                        // })
                    };
                }

                macro_rules! ttype_or_hex {
                    ($v:expr) => {
                        if let Some(ti) = self.trigger_info.triggers.get(&$v) {
                            format!("{} (0x{:x})", ti.name, $v)
                        } else {
                            format!("0x{:x}", $v)
                        }
                    };
                }

                macro_rules! quick_grid {
                    ($ui:expr, $label:expr, $contents:expr) => {
                        egui::Grid::new($label)
                            .num_columns(2)
                            .spacing([40.0, 4.0])
                            .striped(true)
                            .show($ui, $contents);
                    };
                }

                egui::ScrollArea::vertical()
                    .max_height(screen_space.height() - 100.0)
                    .show(ui, |ui| {
                        if let Some(Some(trig)) = self.selected_trigger.map(|v| map.triggers.get(v))
                        {
                            quick_grid!(ui, "t_info", |ui| {
                                readonly_input!(ui, "Type ", ttype_or_hex!(trig.ttype));
                                ui.end_row();
                                readonly_input!(
                                    ui,
                                    "Subtype ",
                                    if let Some(subtype) = trig.tsubtype {
                                        ttype_or_hex!(subtype)
                                    } else {
                                        "None".to_string()
                                    }
                                );
                                ui.end_row();
                                readonly_input!(ui, "Flags ", format!("0x{:x}", trig.game_flags));
                                ui.end_row();

                                ui.label("Position");
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{:.3}, {:.3},  {:.3}",
                                        trig.position.x, trig.position.y, trig.position.z
                                    ));
                                });
                                ui.end_row();

                                ui.label("Rotation");
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{:.3}, {:.3},  {:.3}",
                                        trig.rotation.x.to_degrees(),
                                        trig.rotation.y.to_degrees(),
                                        trig.rotation.z.to_degrees()
                                    ));
                                });
                                ui.end_row();

                                ui.label("Scale");
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{:.2}, {:.2},  {:.2}",
                                        trig.scale.x, trig.scale.y, trig.scale.z
                                    ));
                                });
                                ui.end_row();

                                if let Some(Some(coll)) = trig
                                    .engine_data
                                    .get(3)
                                    .map(|c| c.map(|c| map.trigger_collisions.get(c as usize)))
                                    .flatten()
                                {
                                    ui.label("Collision");
                                    match coll.dtype {
                                        0 => ui.label("Box"),
                                        3 => ui.label("Cylinder"),
                                        u => ui.label(format!(
                                            "{} Unknown collision type {}",
                                            font_awesome::EXCLAMATION_TRIANGLE,
                                            u
                                        )),
                                    };
                                    ui.end_row();
                                }
                            });

                            if !trig.data.is_empty() {
                                ui.separator();
                                ui.strong("Values");
                                quick_grid!(ui, "t_values", |ui| {
                                    for (i, v) in trig.data.iter().enumerate() {
                                        if let Some(v) = v {
                                            let (name, dtype) = if let Some(Some(ti)) = self
                                                .trigger_info
                                                .triggers
                                                .get(&trig.ttype)
                                                .map(|v| v.values.get(&(i as u32)))
                                            {
                                                (ti.name.clone(), ti.dtype)
                                            } else {
                                                (None, TrigDataType::default())
                                            };

                                            readonly_input!(
                                                ui,
                                                name.unwrap_or(format!("#{i} ")),
                                                dtype.to_string(&self.hashcodes, *v)
                                            );
                                            ui.end_row();
                                        }
                                    }
                                });
                            }

                            if trig.engine_data.iter().any(|v| v.is_some()) {
                                ui.separator();
                                ui.strong("Engine values");
                                quick_grid!(ui, "t_extravalues", |ui| {
                                    for (i, v) in trig
                                        .engine_data
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(i, v)| v.as_ref().map(|v| (i, v)))
                                    {
                                        let (name, dtype) = if let Some(ti) =
                                            self.trigger_info.extra_values.get(&(i as u32))
                                        {
                                            (ti.name.clone(), ti.dtype)
                                        } else {
                                            (None, TrigDataType::default())
                                        };

                                        readonly_input!(
                                            ui,
                                            name.unwrap_or(format!("#{i} ")),
                                            dtype.to_string(&self.hashcodes, *v)
                                        );
                                        ui.end_row();
                                    }
                                });
                            }

                            if trig.links.iter().find(|v| **v != -1).is_some() {
                                ui.separator();
                                ui.strong("Outgoing Links");

                                quick_grid!(ui, "t_outlinks", |ui| {
                                    for (i, l) in
                                        trig.links.iter().enumerate().filter(|(_, v)| **v != -1)
                                    {
                                        let ltrig = &map.triggers[*l as usize];
                                        let resp = ui.horizontal(|ui| {
                                            readonly_input!(
                                                ui,
                                                format!("#{i} "),
                                                format!(
                                                    "{} (type {})",
                                                    l,
                                                    ttype_or_hex!(ltrig.ttype)
                                                )
                                            );

                                            if ui
                                                .button(font_awesome::BULLSEYE.to_string())
                                                .clicked()
                                            {
                                                self.go_to_trigger(*l as usize, ltrig)
                                            }
                                        });

                                        if resp.response.hovered() {
                                            self.selected_link = Some(*l);
                                        }

                                        ui.end_row();
                                    }
                                });
                            }

                            if !trig.incoming_links.is_empty() {
                                ui.separator();
                                ui.strong(format!(
                                    "Incoming Links ({} links)",
                                    trig.incoming_links.len()
                                ));

                                for l in trig.incoming_links.iter() {
                                    let ltrig = &map.triggers[*l as usize];
                                    let resp = ui.horizontal(|ui| {
                                        readonly_input!(
                                            ui,
                                            format!("{} (type {})", l, ttype_or_hex!(ltrig.ttype))
                                        );

                                        if ui.button(font_awesome::BULLSEYE.to_string()).clicked() {
                                            self.go_to_trigger(*l as usize, ltrig)
                                        }
                                    });

                                    if resp.response.hovered() {
                                        self.selected_link = Some(*l);
                                    }
                                }
                            }
                        }
                    });
            });
    }

    fn go_to_trigger(&mut self, index: usize, trig: &ProcessedTrigger) {
        self.selected_trigger = Some(index);

        let mut v = self.viewer.lock().unwrap();
        let camera = v.camera_mut();

        self.trigger_focus_tween = Some(Tweeny3D::new(
            tweeny::ease_out_exponential,
            camera.position() + camera.focus_offset(self.trigger_scale),
            trig.position,
            0.5,
        ))
    }
}

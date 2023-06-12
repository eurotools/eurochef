use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

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
        camera::Camera3D,
        entity::EntityRenderer,
        gl_helper,
        trigger::LinkLineRenderer,
        viewer::{BaseViewer, CameraType},
    },
};

pub struct MapFrame {
    pub textures: Vec<RenderableTexture>,
    pub ref_renderers: Vec<Arc<Mutex<EntityRenderer>>>,
    pub placement_renderers: Vec<(u32, EXGeoBaseEntity, Arc<Mutex<EntityRenderer>>)>,
    billboard_renderer: Arc<BillboardRenderer>,
    trigger_texture: glow::Texture,
    link_renderer: Arc<LinkLineRenderer>,
    selected_trigger: usize,

    pub viewer: Arc<Mutex<BaseViewer>>,
    sky_ent: String,

    /// Used to prevent keybinds being triggered while a textfield is focused
    textfield_focused: bool,

    vertex_lighting: bool,
}

const DEFAULT_ICON_DATA: &[u8] = include_bytes!("../../../assets/icons/triggers/default.png");

impl MapFrame {
    pub fn new(
        gl: &glow::Context,
        meshes: &[&ProcessedEntityMesh],
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
            viewer: Arc::new(Mutex::new(BaseViewer::new(gl))),
            sky_ent: String::new(),
            textfield_focused: false,
            vertex_lighting: true,
            billboard_renderer: Arc::new(BillboardRenderer::new(gl).unwrap()),
            link_renderer: Arc::new(LinkLineRenderer::new(gl).unwrap()),
            trigger_texture: unsafe {
                gl_helper::load_texture(
                    gl,
                    default_icon_info.width as i32,
                    default_icon_info.height as i32,
                    &default_icon_data,
                    glow::RGBA,
                    0,
                )
            },
            selected_trigger: 0,
        };

        unsafe {
            for m in meshes {
                let r = Arc::new(Mutex::new(EntityRenderer::new(gl, platform)));
                r.lock().unwrap().load_mesh(gl, m);
                s.ref_renderers.push(r);
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

                let r = Arc::new(Mutex::new(EntityRenderer::new(gl, platform)));
                r.lock().unwrap().load_mesh(gl, m);

                let base = e.base().unwrap().clone();
                s.placement_renderers.push((i, base, r));
            }
        }

        s.placement_renderers
            .sort_by(|(_, e, _), (_, e2, _)| e.sort_value.cmp(&e2.sort_value));

        s
    }

    pub fn show(&mut self, ui: &mut egui::Ui, map: &ProcessedMap) {
        ui.horizontal(|ui| {
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
                    .chain(self.placement_renderers.iter().map(|r| &r.2))
                {
                    r.lock().unwrap().vertex_lighting = self.vertex_lighting;
                }
            }

            ui.add(
                egui::DragValue::new(&mut self.selected_trigger)
                    .speed(0.5)
                    .clamp_range(0..=(map.triggers.len() - 1)),
            );
            ui.label("Selected trigger");
        });

        egui::Frame::canvas(ui.style()).show(ui, |ui| self.show_canvas(ui, map));

        self.viewer.lock().unwrap().show_statusbar(ui);
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui, map: &ProcessedMap) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size() - egui::vec2(0., 16.),
            egui::Sense::click_and_drag(),
        );

        let time: f64 = ui.input(|t| t.time);

        let viewer = self.viewer.clone();
        let (camera_pos, camera_rot) = {
            let mut v = viewer.lock().unwrap();
            if !self.textfield_focused {
                v.update(ui, response);
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
            for r in &renderers {
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

            for r in &renderers {
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

            if let Some(trig) = map.triggers.get(selected_trigger) {
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
                        );
                    }
                }
            }

            for t in &map.triggers {
                billboard_renderer.render(
                    painter.gl(),
                    &viewer.lock().unwrap().uniforms,
                    trigger_texture,
                    t.position,
                    // TODO(cohae): This scaling is too small for spyro
                    0.25,
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

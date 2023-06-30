use std::{io::Seek, sync::Arc};

use anyhow::anyhow;
use egui::{Color32, RichText, Widget};
use eurochef_edb::{
    anim::EXGeoBaseAnimSkin, binrw::BinReaderExt, edb::EdbFile, entity::EXGeoEntity,
    versions::Platform,
};
use eurochef_shared::{
    entities::{read_entity, TriStrip, UXVertex},
    textures::UXGeoTexture,
    IdentifiableResult,
};
use fnv::FnvHashMap;
use font_awesome as fa;
use glam::{Quat, Vec2, Vec3};
use glow::HasContext;

use crate::{
    entity_frame::{EntityFrame, RenderableTexture},
    render::{
        self, camera::ArcBallCamera, entity::EntityRenderer, gl_helper, shaders::Shaders,
        viewer::RenderContext, RenderUniforms,
    },
    strip_ansi_codes,
    textures::cutoff_string,
};

pub struct EntityListPanel {
    gl: Arc<glow::Context>,
    entity_renderer: Option<EntityFrame>,
    entity_label: String,

    entity_previews: FnvHashMap<u32, Option<egui::TextureHandle>>,
    // TODO(cohae): Hack to get shaders for entity previews
    shaders: Shaders,

    entities: Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
    skins: Vec<IdentifiableResult<EXGeoBaseAnimSkin>>,
    ref_entities: Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
    framebuffer: (glow::Framebuffer, glow::Texture),
    framebuffer_msaa: (glow::Framebuffer, glow::Texture),
    textures: Vec<RenderableTexture>,

    /// Preview thumbnail width, in pixels
    preview_size: i32,

    platform: Platform,
}

#[derive(Clone)]
pub struct ProcessedEntityMesh {
    pub vertex_data: Vec<UXVertex>,
    pub indices: Vec<u32>,
    pub strips: Vec<TriStrip>,
    pub flags: u32,
}

impl ProcessedEntityMesh {
    pub fn bounding_box(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for v in &self.vertex_data {
            min = min.min(v.pos.into());
            max = max.max(v.pos.into());
        }

        (min, max)
    }
}

impl EntityListPanel {
    pub fn new(
        ctx: &egui::Context,
        gl: Arc<glow::Context>,
        entities: Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        skins: Vec<IdentifiableResult<EXGeoBaseAnimSkin>>,
        ref_entities: Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
        textures: &[IdentifiableResult<UXGeoTexture>],
        platform: Platform,
    ) -> Self {
        let mut entity_previews = FnvHashMap::default();
        for ires in entities.iter().filter(|ir| ir.data.is_ok()) {
            entity_previews.insert(ires.hashcode, None);
        }
        for ires in skins.iter().filter(|ir| ir.data.is_ok()) {
            entity_previews.insert(ires.hashcode, None);
        }
        for ires in ref_entities.iter().filter(|ir| ir.data.is_ok()) {
            entity_previews.insert(ires.hashcode, None);
        }

        let preview_size = (256.0 * ctx.pixels_per_point()) as i32;

        #[cfg(not(target_family = "wasm"))]
        let framebuffer_msaa = unsafe { Self::create_preview_framebuffer(&gl, true, preview_size) };
        #[cfg(target_family = "wasm")]
        let framebuffer_msaa =
            unsafe { Self::create_preview_framebuffer(&gl, false, preview_size) };

        EntityListPanel {
            framebuffer_msaa,
            framebuffer: unsafe { Self::create_preview_framebuffer(&gl, false, preview_size) },
            textures: Self::load_textures(&gl, textures),
            shaders: Shaders::load_shaders(&gl),
            gl,
            entity_renderer: None,
            entity_label: String::new(),
            entities,
            skins,
            ref_entities,
            entity_previews,
            preview_size,
            platform,
        }
    }

    pub fn load_textures(
        gl: &glow::Context,
        textures: &[IdentifiableResult<UXGeoTexture>],
    ) -> Vec<RenderableTexture> {
        textures
            .iter()
            .map(|it| unsafe {
                if let Ok(t) = &it.data {
                    let mut frames = vec![];

                    for d in &t.frames {
                        let handle = gl_helper::load_texture(
                            gl,
                            t.width as i32,
                            t.height as i32,
                            &d,
                            glow::RGBA,
                            t.flags,
                        );
                        frames.push(handle);
                    }

                    RenderableTexture {
                        frames,
                        framerate: t.framerate as usize,
                        frame_count: t.frame_count as usize,
                        flags: t.flags,
                        // EngineX(T) calculates these as step per frame by dividing each axis by 30000. We're calculating this with seconds instead of frames
                        scroll: Vec2::new(t.scroll[0] as f32 / 500.0, t.scroll[1] as f32 / 500.0),
                        hashcode: it.hashcode,
                    }
                } else {
                    let handle = gl_helper::load_texture(
                        gl,
                        2,
                        2,
                        &[
                            255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255,
                        ],
                        glow::RGBA,
                        0,
                    );

                    RenderableTexture {
                        frames: vec![handle],
                        framerate: 0,
                        frame_count: 0,
                        flags: 0,
                        scroll: Vec2::ZERO,
                        hashcode: it.hashcode,
                    }
                }
            })
            .collect()
    }

    pub fn show(&mut self, context: &egui::Context, ui: &mut egui::Ui) {
        if self.entity_renderer.is_some() {
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    self.entity_renderer = None;
                    return;
                }
                ui.heading(&self.entity_label);
            });
        }

        if let Some(er) = self.entity_renderer.as_mut() {
            ui.separator();
            er.show(ui);
        } else {
            egui::ScrollArea::vertical()
                .id_source("section_scroll_area")
                .always_show_scroll(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if !self.skins.is_empty() {
                        ui.spacing_mut().item_spacing = [16., 2.].into();
                        ui.heading(format!("{} Skeletons", fa::WALKING));
                        ui.spacing_mut().item_spacing = [16., 8.].into();
                        ui.separator();
                        let skin_ids = self
                            .skins
                            .iter()
                            .map(|ir| {
                                (
                                    ir.hashcode,
                                    ir.data.as_ref().err().map(|e| format!("{e:?}")),
                                )
                            })
                            .collect();
                        self.show_section(ui, skin_ids, 2);
                    }

                    if !self.ref_entities.is_empty() {
                        ui.spacing_mut().item_spacing = [16., 2.].into();
                        ui.heading("\u{e52f} Ref Meshes");
                        ui.spacing_mut().item_spacing = [16., 8.].into();
                        ui.separator();
                        let refent_ids = self
                            .ref_entities
                            .iter()
                            .map(|ir| {
                                (
                                    ir.hashcode,
                                    ir.data.as_ref().err().map(|e| format!("{e:?}")),
                                )
                            })
                            .collect();
                        self.show_section(ui, refent_ids, 1);
                    }

                    if !self.entities.is_empty() {
                        ui.spacing_mut().item_spacing = [16., 2.].into();
                        ui.heading(format!("{} Meshes", fa::CUBE));
                        ui.spacing_mut().item_spacing = [16., 8.].into();
                        ui.separator();
                        let entity_ids = self
                            .entities
                            .iter()
                            .map(|ir| {
                                (
                                    ir.hashcode,
                                    ir.data.as_ref().err().map(|e| format!("{e:?}")),
                                )
                            })
                            .collect();
                        self.show_section(ui, entity_ids, 0);
                    }
                });
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.entity_renderer = None;
        }

        if ui.input(|i| i.key_pressed(egui::Key::F5)) {
            self.entity_previews.iter_mut().for_each(|(_, v)| *v = None);
        }

        self.render_previews(context);
    }

    fn show_section(&mut self, ui: &mut egui::Ui, ids: Vec<(u32, Option<String>)>, ty: i32) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = [16., 16.].into();
            for (ii, (i, err)) in ids.iter().enumerate() {
                ui.allocate_ui(egui::Vec2::new(256., 256. + 20.), |ui| {
                    ui.spacing_mut().item_spacing = [4., 4.].into();
                    ui.vertical(|ui| {
                        if let Some(err) = err {
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(256., 256.), egui::Sense::click());

                            ui.painter()
                                .rect_filled(rect, egui::Rounding::none(), Color32::BLACK);

                            ui.painter().text(
                                rect.left_top() + egui::vec2(24., 24.),
                                egui::Align2::CENTER_CENTER,
                                font_awesome::EXCLAMATION_TRIANGLE,
                                egui::FontId::proportional(24.),
                                Color32::RED,
                            );

                            response.on_hover_ui(|ui| {
                                ui.label(format!("Entity {i:x} failed:"));
                                ui.colored_label(
                                    Color32::LIGHT_RED,
                                    cutoff_string(strip_ansi_codes(&err), 1024),
                                );
                            });

                            return;
                        }

                        let response = if let Some(Some(tex)) = self.entity_previews.get(&i) {
                            egui::Image::new(tex.id(), [256., 256.])
                                .uv(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    [1.0, 1.0].into(),
                                ))
                                .sense(egui::Sense::click())
                                .ui(ui)
                        } else {
                            let (rect, response) =
                                ui.allocate_exact_size([256., 256.].into(), egui::Sense::click());

                            ui.painter().rect_filled(
                                rect,
                                egui::Rounding {
                                    nw: 8.,
                                    ne: 8.,
                                    ..Default::default()
                                },
                                Color32::from_rgb(50, 50, 50),
                            );

                            ui.painter().text(
                                rect.center() + [0., 16.].into(),
                                egui::Align2::CENTER_CENTER,
                                fa::CLOCK,
                                egui::FontId::proportional(96.),
                                Color32::WHITE,
                            );

                            response
                        };

                        let response = response.on_hover_ui(|ui| {
                            ui.label(format!("Index: {ii}\nHashcode: {i:08x}"));
                        });

                        if response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.entity_label = match ty {
                                0 => format!("Entity {:x}", i),
                                1 => format!("Ref Entity {}", i),
                                2 => format!("Animation Skin {:x}", i),
                                _ => unreachable!(),
                            };

                            if ty != 2 {
                                self.entity_renderer = Some(EntityFrame::new(
                                    &self.gl,
                                    &[if ty == 0 {
                                        &self
                                            .entities
                                            .iter()
                                            .find(|ir| ir.hashcode == *i)
                                            .as_ref()
                                            .unwrap()
                                            .data
                                            .as_ref()
                                            .unwrap()
                                            .1
                                    } else {
                                        &self
                                            .ref_entities
                                            .iter()
                                            .find(|ir| ir.hashcode == *i)
                                            .as_ref()
                                            .unwrap()
                                            .data
                                            .as_ref()
                                            .unwrap()
                                            .1
                                    }],
                                    &self.textures,
                                    self.platform,
                                ));
                            } else {
                                let mut combined_entities = vec![];
                                let skin = &self
                                    .skins
                                    .iter()
                                    .find(|ir| ir.hashcode == *i)
                                    .as_ref()
                                    .unwrap()
                                    .data
                                    .as_ref()
                                    .unwrap();

                                let entity_indices: Vec<u32> = skin
                                    .entities
                                    .iter()
                                    .chain(skin.more_entities.iter())
                                    .map(|d| d.entity_index & 0x00ffffff)
                                    .collect();

                                for i in entity_indices {
                                    combined_entities
                                        .push(&self.entities[i as usize].data.as_ref().unwrap().1);
                                }

                                self.entity_renderer = Some(EntityFrame::new(
                                    &self.gl,
                                    &combined_entities,
                                    &self.textures,
                                    self.platform,
                                ));
                            }
                        }

                        ui.horizontal(|ui| {
                            match ty {
                                2 => {
                                    ui.colored_label(
                                        egui::Rgba::from_srgba_premultiplied(255, 130, 55, 255),
                                        fa::WALKING.to_string(),
                                    );
                                    ui.label(RichText::new(format!("{i:x}")).strong());
                                }
                                1 => {
                                    ui.colored_label(
                                        egui::Rgba::from_srgba_premultiplied(55, 160, 0, 255),
                                        "\u{e52f}",
                                    );
                                    ui.label(RichText::new(format!("ref_{i}")).strong());
                                }
                                0 => {
                                    ui.colored_label(
                                        egui::Rgba::from_srgba_premultiplied(55, 160, 255, 255),
                                        fa::CUBE.to_string(),
                                    );
                                    ui.label(RichText::new(format!("{i:x}")).strong());
                                }
                                _ => {}
                            };
                        });
                    });
                });
            }
        });
        ui.add_space(16.0);
    }

    #[cfg(not(target_family = "wasm"))]
    const PREVIEW_RENDERS_PER_FRAME: usize = 6;
    #[cfg(target_family = "wasm")]
    const PREVIEW_RENDERS_PER_FRAME: usize = 2;

    fn render_previews(&mut self, context: &egui::Context) {
        for _ in 0..Self::PREVIEW_RENDERS_PER_FRAME {
            if let Some((hc, t)) = self.entity_previews.iter_mut().find(|t| t.1.is_none()) {
                let mut meshes: Vec<&ProcessedEntityMesh> = vec![];

                if let Some(Ok((_, mesh))) = self
                    .entities
                    .iter()
                    .find(|ir| ir.hashcode == *hc)
                    .or(self.ref_entities.iter().find(|ir| ir.hashcode == *hc))
                    .map(|v| v.data.as_ref())
                {
                    meshes.push(mesh)
                } else {
                    if let Some(Ok(skin)) = self
                        .skins
                        .iter()
                        .find(|ir| ir.hashcode == *hc)
                        .map(|v| &v.data)
                    {
                        let entity_indices: Vec<u32> = skin
                            .entities
                            .iter()
                            .chain(skin.more_entities.iter())
                            .map(|d| d.entity_index & 0x00ffffff)
                            .collect();
                        for i in entity_indices {
                            if let Ok((_, mesh)) = &self.entities[i as usize].data.as_ref() {
                                meshes.push(mesh);
                            }
                        }
                    } else {
                        unreachable!("Thumbnail requested for nonexistent entity {hc:x}");
                    }
                }

                let mut bb = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for m in &meshes {
                    let bb2 = m.bounding_box();
                    bb.0 = bb.0.min(bb2.0);
                    bb.1 = bb.1.max(bb2.1);
                }

                let mesh_center = (bb.0 + bb.1) / 2.0;

                let maximum_extent = (bb.1.x - bb.0.x).max(bb.1.y - bb.0.y).max(bb.1.z - bb.0.z);

                let mut out = vec![0u8; (self.preview_size * self.preview_size * 4) as usize];

                let zoom = 0.3 * maximum_extent;
                let mut uniforms = RenderUniforms::default();
                uniforms.update(
                    true,
                    &mut ArcBallCamera::new(Vec3::ZERO, Vec2::new(30., 140.), zoom, false),
                    1.0,
                    0.0,
                );

                unsafe {
                    #[cfg(not(target_family = "wasm"))]
                    self.gl
                        .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer_msaa.0));
                    #[cfg(target_family = "wasm")]
                    self.gl
                        .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer.0));

                    render::start_render(&self.gl);
                    self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
                    self.gl
                        .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                    self.gl.viewport(0, 0, self.preview_size, self.preview_size);

                    let context = RenderContext {
                        shaders: &self.shaders,
                        uniforms: &uniforms,
                    };

                    if meshes.len() == 1 {
                        let mut er = EntityRenderer::new(&self.gl, self.platform);
                        er.load_mesh(&self.gl, meshes[0]);
                        er.draw_both(
                            &self.gl,
                            &context,
                            -mesh_center,
                            Quat::IDENTITY,
                            Vec3::ONE,
                            0.0, // Thumbnails are static so we don't need time
                            &self.textures,
                        );
                    } else {
                        let renderers: Vec<EntityRenderer> = meshes
                            .iter()
                            .map(|m| {
                                let mut er = EntityRenderer::new(&self.gl, self.platform);
                                er.load_mesh(&self.gl, m);
                                er
                            })
                            .collect();

                        for r in &renderers {
                            r.draw_opaque(
                                &self.gl,
                                &context,
                                -mesh_center,
                                Quat::IDENTITY,
                                Vec3::ONE,
                                0.0,
                                &self.textures,
                            );
                        }

                        self.gl.depth_mask(false);

                        for r in &renderers {
                            r.draw_transparent(
                                &self.gl,
                                &context,
                                -mesh_center,
                                Quat::IDENTITY,
                                Vec3::ONE,
                                0.0,
                                &self.textures,
                            );
                        }
                    }

                    // Blit the MSAA framebuffer to a normal one so we can copy it
                    #[cfg(not(target_family = "wasm"))]
                    {
                        self.gl.bind_framebuffer(
                            glow::READ_FRAMEBUFFER,
                            Some(self.framebuffer_msaa.0),
                        );
                        self.gl
                            .bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.framebuffer.0));
                        self.gl.blit_framebuffer(
                            0,
                            0,
                            self.preview_size,
                            self.preview_size,
                            0,
                            0,
                            self.preview_size,
                            self.preview_size,
                            glow::COLOR_BUFFER_BIT,
                            glow::NEAREST,
                        );

                        self.gl
                            .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer.0));
                    }

                    self.gl.read_pixels(
                        0,
                        0,
                        self.preview_size,
                        self.preview_size,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelPackData::Slice(&mut out),
                    );

                    self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                }

                let mut out_flipped = vec![0u8; out.len()];
                for y in 0..self.preview_size {
                    let i = (y * self.preview_size * 4) as usize;
                    let i_flipped = ((self.preview_size - y - 1) * self.preview_size * 4) as usize;
                    out_flipped[i_flipped..i_flipped + self.preview_size as usize * 4]
                        .copy_from_slice(&out[i..i + self.preview_size as usize * 4]);
                }

                let image = egui::ImageData::Color(egui::ColorImage::from_rgba_unmultiplied(
                    [self.preview_size as usize, self.preview_size as usize],
                    &out_flipped,
                ));
                *t = Some(context.load_texture(
                    hc.to_string(),
                    image,
                    egui::TextureOptions::default(),
                ));
            } else {
                break;
            }
        }
    }

    unsafe fn create_preview_framebuffer(
        gl: &glow::Context,
        msaa: bool,
        size: i32,
    ) -> (glow::Framebuffer, glow::Texture) {
        // Create framebuffer object
        let framebuffer = gl
            .create_framebuffer()
            .expect("Failed to create framebuffer");
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

        let texture_target: u32 = if msaa {
            glow::TEXTURE_2D_MULTISAMPLE
        } else {
            glow::TEXTURE_2D
        };

        // Create color texture
        let color_texture = gl.create_texture().expect("Failed to create color texture");
        gl.bind_texture(texture_target, Some(color_texture));
        if msaa {
            gl.tex_image_2d_multisample(texture_target, 4, glow::RGB as i32, size, size, true);
        } else {
            gl.tex_image_2d(
                texture_target,
                0,
                glow::RGB as i32,
                size,
                size,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                None,
            );
        }
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            texture_target,
            Some(color_texture),
            0,
        );

        // Create depth renderbuffer
        let depth_renderbuffer = gl
            .create_renderbuffer()
            .expect("Failed to create depth renderbuffer");
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_renderbuffer));
        if msaa {
            gl.renderbuffer_storage_multisample(
                glow::RENDERBUFFER,
                4,
                glow::DEPTH24_STENCIL8,
                size,
                size,
            );
        } else {
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH24_STENCIL8, size, size);
        }
        gl.bind_renderbuffer(glow::RENDERBUFFER, None);
        gl.framebuffer_renderbuffer(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::RENDERBUFFER,
            Some(depth_renderbuffer),
        );

        // Check framebuffer completeness
        if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
            panic!("Framebuffer is not complete");
        }

        // Unbind framebuffer
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        (framebuffer, color_texture)
    }
}

pub fn read_from_file(
    edb: &mut EdbFile,
) -> anyhow::Result<(
    Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
    Vec<IdentifiableResult<EXGeoBaseAnimSkin>>,
    Vec<IdentifiableResult<(EXGeoEntity, ProcessedEntityMesh)>>,
    Vec<IdentifiableResult<UXGeoTexture>>,
)> {
    let header = edb.header.clone();

    // TODO(cohae): Replace with header iterators
    let mut entities = vec![];
    for e in header.entity_list.iter() {
        let ent = read_entity_identifiable(e.common.address, edb);
        entities.push(IdentifiableResult::new(e.common.hashcode, ent));
    }

    // TODO(cohae): Replace with header iterators?
    let mut refents = vec![];
    for (i, r) in header.refpointer_list.iter().enumerate() {
        edb.seek(std::io::SeekFrom::Start(r.address as u64))?;

        let etype = edb.read_type::<u32>(edb.endian)?;
        if etype == 0x601 || etype == 0x602 || etype == 0x603 {
            let ent = read_entity_identifiable(r.address, edb);
            refents.push(IdentifiableResult::new(i as _, ent));
        }
    }

    let mut skins = vec![];
    for s in header.animskin_list.iter() {
        edb.seek(std::io::SeekFrom::Start(s.common.address as u64))?;

        let skin = edb.read_type_args::<EXGeoBaseAnimSkin>(edb.endian, (edb.header.version,));
        skins.push(IdentifiableResult::new(
            s.common.hashcode,
            match skin {
                Ok(s) => Ok(s),
                Err(e) => Err(anyhow!("Failed to read animskin: {e:?}")),
            },
        ));
    }

    let textures = UXGeoTexture::read_all(edb);

    Ok((entities, skins, refents, textures))
}

fn read_entity_identifiable(
    address: u32,
    edb: &mut EdbFile,
) -> anyhow::Result<(EXGeoEntity, ProcessedEntityMesh)> {
    edb.seek(std::io::SeekFrom::Start(address as u64))?;

    let ent = edb.read_type_args(edb.endian, (edb.header.version, edb.platform))?;

    let mut vertex_data = vec![];
    let mut indices = vec![];
    let mut strips = vec![];
    read_entity(
        &ent,
        &mut vertex_data,
        &mut indices,
        &mut strips,
        edb.endian,
        edb.header.version,
        edb.platform,
        edb,
        4,
        false,
        false,
    )?;

    let flags = ent.base().map(|b| b.flags).unwrap_or_default();

    Ok((
        ent,
        ProcessedEntityMesh {
            vertex_data,
            indices,
            strips,
            flags,
        },
    ))
}

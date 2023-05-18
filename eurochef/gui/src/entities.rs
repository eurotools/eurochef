use std::{
    io::{Read, Seek},
    sync::Arc,
};

use egui::{Color32, RichText, Vec2, Widget};
use eurochef_edb::{
    anim::EXGeoBaseAnimSkin,
    binrw::{BinReaderExt, Endian},
    entity::EXGeoEntity,
    header::EXGeoHeader,
    versions::Platform,
};
use eurochef_shared::entities::{read_entity, TriStrip, UXVertex};
use fnv::FnvHashMap;
use font_awesome as fa;

use crate::entity_renderer::EntityFrame;

pub struct EntityListPanel {
    gl: Arc<glow::Context>,
    // TODO: replace with drawing clock icon from FA dynamically
    missing_texture: egui::TextureHandle,
    entity_renderer: Option<EntityFrame>,

    entity_previews: FnvHashMap<String, Option<egui::TextureHandle>>,

    entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
    skins: Vec<(u32, EXGeoBaseAnimSkin)>,
    ref_entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
}

pub struct ProcessedEntityMesh {
    pub vertex_data: Vec<UXVertex>,
    pub indices: Vec<u32>,
    pub strips: Vec<TriStrip>,
}

impl EntityListPanel {
    pub fn new(
        ctx: &egui::Context,
        gl: Arc<glow::Context>,
        entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
        skins: Vec<(u32, EXGeoBaseAnimSkin)>,
        ref_entities: Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
    ) -> Self {
        const MAGENTA_CHECKER: [u8; 4 * 4] = [
            255, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 255, 255,
        ];

        let texture = ctx.load_texture(
            "argh",
            egui::ColorImage::from_rgba_unmultiplied([2, 2], &MAGENTA_CHECKER),
            egui::TextureOptions::NEAREST,
        );

        let mut entity_previews = FnvHashMap::default();
        for (hashcode, _, _) in entities.iter() {
            entity_previews.insert(format!("{hashcode:x}"), None);
        }
        for (hashcode, _) in skins.iter() {
            entity_previews.insert(format!("{hashcode:x}"), None);
        }
        for (index, _, _) in ref_entities.iter() {
            entity_previews.insert(format!("ref_{index}"), None);
        }

        EntityListPanel {
            gl,
            missing_texture: texture,
            entity_renderer: None,
            entities,
            skins,
            ref_entities,
            entity_previews,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.entity_renderer.is_some() {
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    self.entity_renderer = None;
                    return;
                }
                ui.heading(format!(
                    "{:x}",
                    self.entity_renderer.as_ref().unwrap().hashcode
                ));
            });
        }

        if let Some(er) = self.entity_renderer.as_mut() {
            ui.separator();

            unsafe {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    er.show(ui, self.gl.clone());
                });
            }
        } else {
            egui::ScrollArea::vertical()
                .id_source("section_scroll_area")
                .always_show_scroll(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = [16., 2.].into();
                    ui.heading(format!("{} Skeletons", fa::WALKING));
                    ui.spacing_mut().item_spacing = [16., 8.].into();
                    ui.separator();
                    let skin_ids = self.skins.iter().map(|(v, _)| *v).collect();
                    self.show_things(ui, skin_ids, 2);

                    ui.spacing_mut().item_spacing = [16., 2.].into();
                    ui.heading("\u{e52f} Ref Meshes");
                    ui.spacing_mut().item_spacing = [16., 8.].into();
                    ui.separator();
                    let refent_ids = self.ref_entities.iter().map(|(v, _, _)| *v).collect();
                    self.show_things(ui, refent_ids, 1);

                    ui.spacing_mut().item_spacing = [16., 2.].into();
                    ui.heading(format!("{} Meshes", fa::CUBE));
                    ui.spacing_mut().item_spacing = [16., 8.].into();
                    ui.separator();
                    let entity_ids = self.entities.iter().map(|(v, _, _)| *v).collect();
                    self.show_things(ui, entity_ids, 0);
                });
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.entity_renderer = None;
        }
    }

    fn show_things(&mut self, ui: &mut egui::Ui, ids: Vec<u32>, ty: i32) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = [16., 16.].into();
            for i in ids {
                ui.allocate_ui(Vec2::splat(256.), |ui| {
                    ui.spacing_mut().item_spacing = [4., 4.].into();
                    ui.vertical(|ui| {
                        let label = match ty {
                            0 | 2 => {
                                format!("{i:x}")
                            }
                            1 => {
                                format!("ref_{i}")
                            }
                            _ => unreachable!(),
                        };

                        let response = if let Some(tex) = self.entity_previews.get(&label).unwrap()
                        {
                            egui::Image::new(tex.id(), [256., 256. - 22.])
                                .sense(egui::Sense::click())
                                .ui(ui)
                        } else {
                            let (rect, response) = ui.allocate_exact_size(
                                [256., 256. - 22.].into(),
                                egui::Sense::click(),
                            );

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

                        if response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            if ty != 2 {
                                self.entity_renderer = Some(EntityFrame::new(
                                    &self.gl,
                                    i,
                                    if ty == 0 {
                                        &self.entities.iter().find(|(v, _, _)| *v == i).unwrap().2
                                    } else {
                                        &self
                                            .ref_entities
                                            .iter()
                                            .find(|(v, _, _)| *v == i)
                                            .unwrap()
                                            .2
                                    },
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
}

// TODO(cohae): EdbFile struct so we dont have to read endianness separately
pub fn read_from_file<R: Read + Seek>(
    reader: &mut R,
    platform: Platform,
) -> (
    Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
    Vec<(u32, EXGeoBaseAnimSkin)>,
    Vec<(u32, EXGeoEntity, ProcessedEntityMesh)>,
) {
    reader.seek(std::io::SeekFrom::Start(0)).ok();
    let endian = if reader.read_ne::<u8>().unwrap() == 0x47 {
        Endian::Big
    } else {
        Endian::Little
    };
    reader.seek(std::io::SeekFrom::Start(0)).unwrap();

    let header = reader
        .read_type::<EXGeoHeader>(endian)
        .expect("Failed to read header");

    // TODO(cohae): Replace with header iterators
    let mut entities = vec![];
    for e in header.entity_list.iter() {
        reader
            .seek(std::io::SeekFrom::Start(e.common.address as u64))
            .unwrap();

        let ent = reader
            .read_type_args(endian, (header.version, platform))
            .unwrap();

        let mut vertex_data = vec![];
        let mut indices = vec![];
        let mut strips = vec![];
        if let Err(err) = read_entity(
            &ent,
            &mut vertex_data,
            &mut indices,
            &mut strips,
            endian,
            header.version,
            platform,
            reader,
            4,
            false,
        ) {
            error!("Failed to extract entity: {err}");
            continue;
        }

        entities.push((
            e.common.hashcode,
            ent,
            ProcessedEntityMesh {
                vertex_data,
                indices,
                strips,
            },
        ));
    }

    // TODO(cohae): Replace with header iterators?
    let mut refents = vec![];
    for (i, r) in header.refpointer_list.iter().enumerate() {
        reader
            .seek(std::io::SeekFrom::Start(r.address as u64))
            .unwrap();
        let etype = reader.read_type::<u32>(endian).unwrap();

        if etype == 0x601 || etype == 0x603 {
            reader
                .seek(std::io::SeekFrom::Start(r.address as u64))
                .unwrap();

            let ent = reader
                .read_type_args(endian, (header.version, platform))
                .unwrap();

            let mut vertex_data = vec![];
            let mut indices = vec![];
            let mut strips = vec![];
            if let Err(err) = read_entity(
                &ent,
                &mut vertex_data,
                &mut indices,
                &mut strips,
                endian,
                header.version,
                platform,
                reader,
                4,
                false,
            ) {
                error!("Failed to extract entity: {err}");
                continue;
            }

            refents.push((
                i as u32,
                ent,
                ProcessedEntityMesh {
                    vertex_data,
                    indices,
                    strips,
                },
            ));
        }
    }

    let mut skins = vec![];
    for s in header.animskin_list.iter() {
        reader
            .seek(std::io::SeekFrom::Start(s.common.address as u64))
            .unwrap();
        skins.push((
            s.common.hashcode,
            reader.read_type_args(endian, (header.version,)).unwrap(),
        ));
    }

    (entities, skins, refents)
}

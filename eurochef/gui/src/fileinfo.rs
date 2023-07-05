use egui::RichText;
use egui_extras::Column;
use eurochef_shared::maps::format_hashcode;
use font_awesome as fa;

use eurochef_edb::{header::EXGeoHeader, Hashcode};
use nohash_hasher::IntMap;

use crate::render::RenderStore;

pub struct FileInfoPanel {
    pub header: EXGeoHeader,
    pub external_references: Vec<(Hashcode, Hashcode)>,
}

impl FileInfoPanel {
    pub fn new(header: EXGeoHeader) -> Self {
        Self {
            header,
            external_references: vec![],
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        hashcodes: &IntMap<Hashcode, String>,
        render_store: &RenderStore,
    ) {
        macro_rules! quick_info {
            ($label:expr, $value:expr) => {
                ui.horizontal(|ui| {
                    ui.strong(format!("{}:", $label));
                    ui.label($value);
                })
            };
        }
        macro_rules! quick_array {
            ($label:expr, $array:ident) => {
                let v = self.header.$array.len();
                quick_info!(
                    $label,
                    if v == 0 {
                        "empty".to_string()
                    } else {
                        if v > 1 {
                            format!("{} entries", v)
                        } else {
                            "1 entry".to_string()
                        }
                    }
                )
            };
        }

        ui.label(egui::RichText::new(format!("{} EDB File Info", fa::INFO_CIRCLE)).heading());
        quick_info!("Version", self.header.version.to_string());
        ui.horizontal(|ui| {
            ui.strong("Hashcode:");
            ui.label(format!("{:x}", self.header.hashcode));
            if ui.button(font_awesome::CLIPBOARD.to_string()).clicked() {
                ui.output_mut(|o| o.copied_text = format!("{:x}", self.header.hashcode));
            }
        });

        quick_info!("Flags", format!("0x{:08x}", self.header.flags));

        quick_info!(
            "Build timestamp",
            format!(
                "{}",
                chrono::NaiveDateTime::from_timestamp_opt(self.header.time as i64, 0).unwrap()
            )
        );
        quick_info!(
            "Base file size",
            format!("{}KB", self.header.base_file_size / 1024)
        );
        quick_info!("File size", format!("{}KB", self.header.file_size / 1024));

        ui.separator();

        quick_array!("Sections", section_list);
        quick_array!("Refpointers", refpointer_list);
        quick_array!("Entities", entity_list);
        quick_array!("Animations", anim_list);
        quick_array!("Animation skins", animskin_list);
        quick_array!("Animation scripts", animscript_list);
        quick_array!("Maps", map_list);
        quick_array!("Animation modes", animmode_list);
        quick_array!("Animation sets", animset_list);
        quick_array!("Particles", particle_list);
        quick_array!("Swooshes", swoosh_list);
        quick_array!("Spreadsheets", spreadsheet_list);
        quick_array!("Fonts", font_list);
        quick_array!("Force feedback", forcefeedback_list);
        quick_array!("Materials", material_list);
        quick_array!("Textures", texture_list);
        quick_array!("unk_c0", unk_c0);

        ui.add_space(16.0);
        ui.label(egui::RichText::new(format!("{} External references", fa::LINK)).heading());
        egui::ScrollArea::vertical().show(ui, |ui| {
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
            let table = egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(256.))
                .column(Column::exact(356.))
                .column(Column::initial(16.0).at_most(32.0))
                .min_scrolled_height(0.0);

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("File");
                    });
                    header.col(|ui| {
                        ui.strong("Hashcode");
                    });
                    header.col(|ui| {
                        ui.strong("Loaded");
                    });
                })
                .body(|body| {
                    body.rows(
                        text_height,
                        self.external_references.len(),
                        |row_index, mut row| {
                            let (file_hashcode, object_hashcode) =
                                &self.external_references[row_index];
                            row.col(|ui| {
                                ui.label(format_hashcode(hashcodes, *file_hashcode));
                            });

                            row.col(|ui| {
                                ui.label(format_hashcode(hashcodes, *object_hashcode));
                            });

                            row.col(|ui| {
                                if format_hashcode(hashcodes, *object_hashcode)
                                    .starts_with("HT_Animation")
                                {
                                    ui.label(
                                        RichText::new(font_awesome::MINUS)
                                            .color(egui::Color32::GOLD),
                                    );
                                } else {
                                    let loaded = render_store
                                        .is_object_loaded(*file_hashcode, *object_hashcode);
                                    ui.label(
                                        RichText::new(if loaded {
                                            font_awesome::CHECK
                                        } else {
                                            '\u{f00d}'
                                        })
                                        .color(
                                            if loaded {
                                                egui::Color32::GREEN
                                            } else {
                                                egui::Color32::RED
                                            },
                                        ),
                                    );
                                }
                            });
                        },
                    )
                });
        });
    }
}

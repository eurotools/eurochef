use std::io::{Read, Seek};

use egui::FontSelection;
use eurochef_edb::{
    binrw::{BinReaderExt, Endian},
    header::EXGeoHeader,
};
use eurochef_shared::spreadsheets::{UXGeoSpreadsheet, UXGeoTextItem};

pub struct TextItemList {
    /// Search query for a specific hashcode
    search_hashcode: String,
    /// Search query for text item contents
    search_text: String,

    spreadsheet: UXGeoSpreadsheet,
    selected_section: usize,
}

impl TextItemList {
    pub fn new(spreadsheet: UXGeoSpreadsheet) -> Self {
        Self {
            search_hashcode: String::new(),
            search_text: String::new(),
            spreadsheet,
            selected_section: 0,
        }
    }

    // TODO: Display separate spreadsheets
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search: ");
            ui.text_edit_singleline(&mut self.search_text);
            ui.label("Hashcode: ");
            egui::TextEdit::singleline(&mut self.search_hashcode)
                .font(FontSelection::Style(egui::TextStyle::Monospace))
                .desired_width(76.0)
                .show(ui);
        });

        ui.separator();

        let filtered_items: Vec<Vec<&UXGeoTextItem>> = self
            .spreadsheet
            .0
            .iter()
            .map(|s| {
                s.entries
                    .iter()
                    .filter(|v| {
                        if self.search_hashcode.is_empty() {
                            true
                        } else {
                            format!("{:08x}", v.hashcode)
                                .contains(&self.search_hashcode.to_lowercase())
                        }
                    })
                    .filter(|v| {
                        v.text
                            .to_lowercase()
                            .contains(&self.search_text.to_lowercase())
                    })
                    .collect()
            })
            .collect();

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_source("section_scroll_area")
                    .always_show_scroll(true)
                    .show(ui, |ui| {
                        let mut current_set = 0;
                        for (i, s) in self.spreadsheet.0.iter().enumerate() {
                            if filtered_items[i].is_empty() {
                                continue;
                            }

                            if (s.hashcode & 0xffff0000) != current_set && s.hashcode != u32::MAX {
                                ui.label(format!("Set {:08x}", s.hashcode & 0xffff0000));
                            }

                            if s.hashcode != u32::MAX {
                                current_set = s.hashcode & 0xffff0000;
                            }

                            ui.selectable_value(
                                &mut self.selected_section,
                                i,
                                format!("  Section {:08x}", s.hashcode),
                            );
                        }
                    });
            });

            ui.vertical(|ui| {
                let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 1.25;
                let table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::exact(76.0))
                    .column(egui_extras::Column::exact(76.0))
                    .column(egui_extras::Column::remainder().resizable(true).clip(true));

                table
                    .header(20., |mut header| {
                        header.col(|ui| {
                            ui.strong("Hashcode");
                        });
                        header.col(|ui| {
                            ui.strong("Sound");
                        });
                        header.col(|ui| {
                            ui.strong("Text");
                        });
                    })
                    .body(|body| {
                        body.rows(
                            text_height,
                            filtered_items[self.selected_section].len(),
                            |row_index, mut row| {
                                let item = filtered_items[self.selected_section][row_index];
                                let context_menu = |ui: &mut egui::Ui| {
                                    if ui.button("Copy hashcode").clicked() {
                                        ui.output_mut(|o| {
                                            o.copied_text = format!("{:08x}", item.hashcode)
                                        });
                                        ui.close_menu()
                                    }
                                    if ui.button("Copy text").clicked() {
                                        ui.output_mut(|o| o.copied_text = item.text.clone());
                                        ui.close_menu()
                                    }
                                };

                                row.col(|ui| {
                                    ui.label(format!("0x{:x}", item.hashcode));
                                })
                                .1
                                .context_menu(context_menu);

                                row.col(|ui| {
                                    if item.sound_hashcode == u32::MAX {
                                        ui.label("none");
                                    } else {
                                        ui.label(format!("0x{:x}", item.sound_hashcode));
                                    }
                                })
                                .1
                                .context_menu(context_menu);

                                row.col(|ui| {
                                    ui.style_mut().wrap = Some(false);
                                    ui.label(&item.text.replace('\n', "\\n"));
                                })
                                .1
                                .context_menu(context_menu);
                            },
                        )
                    });
            });
        });
    }
}

// TODO(cohae): EdbFile struct so we dont have to read endianness separately
pub fn read_from_file<R: Read + Seek>(reader: &mut R) -> Vec<UXGeoSpreadsheet> {
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

    UXGeoSpreadsheet::read_all(header, reader, endian)
}

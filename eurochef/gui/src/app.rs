use egui::{Color32, NumExt};

use crate::{fileinfo, spreadsheet, textures};

/// Basic app tracking state
pub enum AppState {
    Ready,
    Loading(String),
    Error(anyhow::Error),
}

#[derive(PartialEq)]
enum Panel {
    FileInfo,
    Textures,
    Spreadsheets,
}

pub struct EurochefApp {
    state: AppState,
    current_panel: Panel,

    spreadsheetlist: Option<spreadsheet::TextItemList>,
    fileinfo: Option<fileinfo::FileInfoPanel>,
    textures: Option<textures::TextureList>,
}

impl Default for EurochefApp {
    fn default() -> Self {
        Self {
            state: AppState::Ready,
            current_panel: Panel::FileInfo,
            spreadsheetlist: None,
            fileinfo: None,
            textures: None,
        }
    }
}

impl EurochefApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for EurochefApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            state,
            current_panel,
            spreadsheetlist,
            fileinfo,
            textures,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        // super::web::import_data();

                        // TODO(cohae): drag and drop loading
                        // TODO(cohae): async loading (will allow WASM support)
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Eurocom DB", &["edb"])
                            .pick_file()
                        {
                            let mut file = std::fs::File::open(path).unwrap();
                            *fileinfo = Some(fileinfo::FileInfoPanel::new(
                                fileinfo::read_from_file(&mut file),
                            ));

                            let spreadsheets = spreadsheet::read_from_file(&mut file);
                            if spreadsheets.len() > 0 {
                                *spreadsheetlist =
                                    Some(spreadsheet::TextItemList::new(spreadsheets[0].clone()));
                            }

                            *textures = Some(textures::TextureList::new(textures::read_from_file(
                                &mut file,
                            )));

                            textures.as_mut().unwrap().load_textures(ctx);
                        }
                    }
                });
            });
        });

        // Run the app at refresh rate on the texture panel (for animated textures)
        match current_panel {
            Panel::Textures => ctx.request_repaint(),
            _ => {
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.));
            }
        }

        match state {
            AppState::Ready => {}
            AppState::Loading(s) => {
                let screen_rect = ctx.screen_rect();
                let max_height = 320.0.at_most(screen_rect.height());
                egui::Window::new("Loading")
                    .title_bar(false)
                    .pivot(egui::Align2::CENTER_TOP)
                    .fixed_pos(screen_rect.center() - 0.5 * max_height * egui::Vec2::Y)
                    .frame(egui::Frame::window(&ctx.style()).inner_margin(16.))
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(s.as_str());
                        });
                    });
            }
            AppState::Error(e) => {
                let screen_rect = ctx.screen_rect();
                let max_height = 320.0.at_most(screen_rect.height());
                let mut open = true;
                egui::Window::new("Error")
                    .pivot(egui::Align2::CENTER_TOP)
                    .fixed_pos(screen_rect.center() - 0.5 * max_height * egui::Vec2::Y)
                    // .frame(egui::Frame::window(&ctx.style()).inner_margin(16.))
                    .resizable(false)
                    .collapsible(false)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            let icon = egui::RichText::new("❗")
                                .color(Color32::from_rgb(200, 90, 90))
                                .size(30.);

                            ui.label(icon);
                            ui.label(format!("{e}"));
                        });

                        if !e.backtrace().to_string().starts_with("disabled backtrace") {
                            ui.collapsing("Backtrace", |ui| {
                                egui::ScrollArea::vertical()
                                    .show(ui, |ui| ui.label(e.backtrace().to_string()));
                            });
                        }
                    });

                if !open {
                    *state = AppState::Ready;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if fileinfo.is_none() {
                ui.heading("No file loaded");
                return;
            }

            ui.horizontal(|ui| {
                if fileinfo.is_some() {
                    ui.selectable_value(current_panel, Panel::FileInfo, "File info");
                }

                if textures.is_some() {
                    ui.selectable_value(current_panel, Panel::Spreadsheets, "Spreadsheets");
                }

                if spreadsheetlist.is_some() {
                    ui.selectable_value(current_panel, Panel::Textures, "Textures");
                }
            });
            ui.separator();

            match current_panel {
                Panel::FileInfo => fileinfo.as_mut().map(|s| s.show(ui)),
                Panel::Textures => textures.as_mut().map(|s| s.show(ui)),
                Panel::Spreadsheets => spreadsheetlist.as_mut().map(|s| s.show(ui)),
            };
        });

        match current_panel {
            Panel::Textures => textures.as_mut().map(|s| s.show_enlarged_window(ctx)),
            _ => None,
        };
    }
}
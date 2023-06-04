use std::{
    fs::File,
    io::{Cursor, Read, Seek},
    sync::Arc,
};

use crossbeam::atomic::AtomicCell;
use eframe::CreationContext;
use egui::{Color32, FontData, FontDefinitions, NumExt};
use eurochef_edb::versions::Platform;
use glow::HasContext;

use crate::{entities, fileinfo, maps, spreadsheet, textures};

/// Basic app tracking state
pub enum AppState {
    Ready,
    SelectPlatform,
    Loading(String),
    Error(anyhow::Error),
}

#[derive(PartialEq)]
enum Panel {
    FileInfo,
    Maps,
    Entities,
    Textures,
    Spreadsheets,
}

pub struct EurochefApp {
    gl: Arc<glow::Context>,

    state: AppState,
    current_panel: Panel,

    spreadsheetlist: Option<spreadsheet::TextItemList>,
    fileinfo: Option<fileinfo::FileInfoPanel>,
    textures: Option<textures::TextureList>,
    entities: Option<entities::EntityListPanel>,
    maps: Option<maps::MapViewerPanel>,

    load_input: Arc<AtomicCell<Option<(Vec<u8>, String)>>>,
    pending_file: Option<(Vec<u8>, Option<Platform>)>,
    selected_platform: Platform,

    ps2_warning: bool,
}

impl EurochefApp {
    /// Called once before the first frame.
    pub fn new(path: Option<String>, cc: &CreationContext<'_>) -> Self {
        // Install FontAwesome font and place it second
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "font_awesome".to_owned(),
            FontData::from_static(include_bytes!("../assets/FontAwesomeSolid.ttf")),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "font_awesome".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        #[cfg(not(target_arch = "wasm32"))]
        unsafe {
            let gl = cc.gl.as_ref().unwrap();

            gl.enable(glow::DEBUG_OUTPUT);
            gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
            gl.debug_message_callback(|source, ty, id, severity, msg| {
                println!("OpenGL s={source} t={ty} i={id} s={severity}: {msg}");
            });
            gl.debug_message_control(glow::DONT_CARE, glow::DONT_CARE, glow::DONT_CARE, &[], true);
        }

        let mut s = Self {
            gl: cc.gl.clone().unwrap(),
            state: AppState::Ready,
            current_panel: Panel::FileInfo,
            spreadsheetlist: None,
            fileinfo: None,
            textures: None,
            entities: None,
            maps: None,
            load_input: Arc::new(AtomicCell::new(None)),
            pending_file: None,
            selected_platform: Platform::Ps2,
            ps2_warning: false,
        };

        if let Some(path) = path {
            s.load_file_with_path(path);
        }

        s
    }

    // TODO: Error handling
    pub fn load_file_with_path<P: AsRef<std::path::Path>>(&mut self, path: P) {
        let platform = Platform::from_path(&path);

        let mut f = File::open(path).unwrap();
        let mut data = vec![];
        f.read_to_end(&mut data).unwrap();
        self.pending_file = Some((data, platform));
    }

    // TODO: Error handling
    pub fn load_file<R: Read + Seek>(
        &mut self,
        platform: Platform,
        reader: &mut R,
        ctx: &egui::Context,
    ) {
        if platform == Platform::Ps2 {
            self.ps2_warning = true;
        }

        self.current_panel = Panel::FileInfo;
        self.spreadsheetlist = None;
        self.fileinfo = None;
        self.textures = None;

        // TODO(cohae): should loader functions be in the struct impls?
        self.fileinfo = Some(fileinfo::FileInfoPanel::new(fileinfo::read_from_file(
            reader,
        )));

        let spreadsheets = spreadsheet::read_from_file(reader);
        if spreadsheets.len() > 0 {
            self.spreadsheetlist = Some(spreadsheet::TextItemList::new(spreadsheets[0].clone()));
        }

        if [
            Platform::Xbox,
            Platform::Xbox360,
            Platform::Pc,
            Platform::Ps2,
        ]
        .contains(&platform)
        {
            let (entities, skins, ref_entities, textures) =
                entities::read_from_file(reader, platform);
            if entities.len() + skins.len() + ref_entities.len() > 0 {
                self.entities = Some(entities::EntityListPanel::new(
                    ctx,
                    self.gl.clone(),
                    entities.clone(),
                    skins,
                    ref_entities.clone(),
                    &textures,
                ));

                if self.fileinfo.as_ref().unwrap().header.map_list.len() > 0 {
                    let map = maps::read_from_file(reader, platform);
                    self.maps = Some(maps::MapViewerPanel::new(
                        ctx,
                        self.gl.clone(),
                        map,
                        entities,
                        ref_entities,
                        &textures,
                    ));
                }
            }
        } else {
            self.entities = None;
        }

        self.textures = Some(textures::TextureList::new(textures::read_from_file(
            reader, platform,
        )));

        self.textures.as_mut().unwrap().load_textures(ctx);

        self.state = AppState::Ready;
    }
}

impl eframe::App for EurochefApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some((data, load_path)) = self.load_input.take() {
            let platform = Platform::from_path(&load_path);
            self.pending_file = Some((data, platform));
        }

        if let Some((data, platform)) = self.pending_file.as_ref() {
            if let Some(platform) = platform {
                info!(
                    "Loading file of {} bytes with platform {platform}",
                    data.len()
                );
                let mut cur = Cursor::new(data.clone()); // FIXME: Cloning the data hurts my soul
                self.load_file(*platform, &mut cur, ctx);
                self.pending_file = None;
            } else {
                self.state = AppState::SelectPlatform;
            }
        }

        let Self {
            state,
            current_panel,
            spreadsheetlist,
            fileinfo,
            textures,
            load_input,
            entities,
            selected_platform,
            maps,
            ..
        } = self;

        let load_clone = load_input.clone();
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        // TODO(cohae): drag and drop loading
                        #[cfg(target_arch = "wasm32")]
                        {
                            wasm_bindgen_futures::spawn_local(async move {
                                let future = rfd::AsyncFileDialog::new()
                                    .add_filter("Eurocom DB", &["edb"])
                                    .pick_file();
                                if let Some(file) = future.await {
                                    let data = file.read().await;
                                    info!("{}", file.file_name());
                                    load_clone.store(Some((data, file.file_name())));
                                } else {
                                }
                            });
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        std::thread::spawn(move || {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("EngineX Database", &["edb"])
                                .pick_file()
                            {
                                let mut f = File::open(&path).unwrap();
                                let mut data = vec![];
                                f.read_to_end(&mut data).unwrap();

                                load_clone.store(Some((data, path.to_string_lossy().to_string())));
                            } else {
                                load_clone.store(None);
                            }
                        });

                        ui.close_menu()
                    }
                });
            });
        });

        // Run the app at refresh rate on the texture panel (for animated textures)
        match current_panel {
            Panel::Entities | Panel::Textures | Panel::Maps => ctx.request_repaint(),
            _ => {
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(1.));
            }
        }

        let screen_rect = ctx.screen_rect();
        let max_height = 320.0.at_most(screen_rect.height());

        // TODO(cohae): More generic dialog (use for loading and error)
        if self.ps2_warning {
            egui::Window::new("PS2 Support")
            .pivot(egui::Align2::CENTER_TOP)
            .fixed_pos(screen_rect.center() - 0.5 * max_height * egui::Vec2::Y)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(16.))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (irect, _) =
                        ui.allocate_exact_size([54., 54.].into(), egui::Sense::hover());
                    ui.painter().text(
                        irect.center() + [0., 8.].into(),
                        egui::Align2::CENTER_CENTER,
                        font_awesome::EXCLAMATION_TRIANGLE,
                        egui::FontId::proportional(48.),
                        Color32::from_rgb(249, 239, 40),
                    );

                    ui.label("PS2 support is currently highly experimental.\nTextures work, but most entities will not draw properly.");
                });
                if ui.button("I understand").clicked() {
                    self.ps2_warning = false;
                }
            });
        }

        match state {
            AppState::Ready => {}
            AppState::Loading(s) => {
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
            AppState::SelectPlatform => {
                egui::Window::new("Select platform")
                    .title_bar(false)
                    .pivot(egui::Align2::CENTER_TOP)
                    .fixed_pos(screen_rect.center() - 0.5 * max_height * egui::Vec2::Y)
                    .frame(egui::Frame::window(&ctx.style()).inner_margin(16.))
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.heading("Please select the platform for this file");
                        ui.separator();
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.strong("Platform: ");
                            egui::ComboBox::from_label("")
                                .selected_text(selected_platform.to_string())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        selected_platform,
                                        Platform::GameCube,
                                        "GameCube",
                                    );
                                    ui.selectable_value(selected_platform, Platform::Pc, "PC");
                                    ui.selectable_value(
                                        selected_platform,
                                        Platform::Ps2,
                                        "Playstation 2",
                                    );
                                    ui.selectable_value(
                                        selected_platform,
                                        Platform::Ps3,
                                        "Playstation 3",
                                    );
                                    ui.selectable_value(
                                        selected_platform,
                                        Platform::ThreeDS,
                                        "3DS",
                                    );
                                    ui.selectable_value(selected_platform, Platform::Wii, "Wii");
                                    ui.selectable_value(selected_platform, Platform::WiiU, "Wii U");
                                    ui.selectable_value(selected_platform, Platform::Xbox, "Xbox");
                                    ui.selectable_value(
                                        selected_platform,
                                        Platform::Xbox360,
                                        "Xbox 360",
                                    );
                                });
                        });

                        ui.horizontal(|ui| {
                            if ui.button("Load").clicked() {
                                if let Some((_, platform)) = self.pending_file.as_mut() {
                                    *platform = Some(*selected_platform);
                                }
                                self.state = AppState::Loading("Loading file".to_string());
                            }

                            if ui.button("Cancel").clicked() {
                                self.pending_file = None;
                                self.state = AppState::Ready;
                            }
                        });
                    });
            }
            AppState::Error(e) => {
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
                            let (irect, _) =
                                ui.allocate_exact_size([48., 48.].into(), egui::Sense::hover());
                            ui.painter().text(
                                irect.center() + [0., 8.].into(),
                                egui::Align2::CENTER_CENTER,
                                '\u{f00d}',
                                egui::FontId::proportional(48.),
                                Color32::from_rgb(250, 40, 40),
                            );

                            ui.label(format!("{e}"));
                        });

                        if !e.backtrace().to_string().starts_with("disabled backtrace") {
                            ui.add_space(4.);
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

                if spreadsheetlist.is_some() {
                    ui.selectable_value(current_panel, Panel::Spreadsheets, "Spreadsheets");
                }

                if entities.is_some() {
                    ui.selectable_value(current_panel, Panel::Entities, "Entities");
                }

                if textures.is_some() {
                    ui.selectable_value(current_panel, Panel::Textures, "Textures");
                }

                if maps.is_some() {
                    ui.selectable_value(current_panel, Panel::Maps, "Maps");
                }
            });
            ui.separator();

            match current_panel {
                Panel::FileInfo => fileinfo.as_mut().map(|s| s.show(ui)),
                Panel::Textures => textures.as_mut().map(|s| s.show(ui)),
                Panel::Entities => entities.as_mut().map(|s| s.show(ctx, ui)),
                Panel::Spreadsheets => spreadsheetlist.as_mut().map(|s| s.show(ui)),
                Panel::Maps => maps.as_mut().map(|s| s.show(ctx, ui)),
            };
        });

        match current_panel {
            Panel::Textures => textures.as_mut().map(|s| s.show_enlarged_window(ctx)),
            _ => None,
        };
    }
}

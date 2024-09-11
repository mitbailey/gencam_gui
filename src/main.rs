#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::any::Any;
use std::vec;
use std::collections::HashMap;

use eframe::egui;
use eframe::egui::{Margin, Visuals};
use egui::menu;
use egui::{Frame, Widget, Id};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Generic Camera GUI",
        native_options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<GenCamGUI>::default())
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| {
                    // This gives us image support:
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    Ok(Box::<Mcs>::default())
                }),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

enum GUITabKind {
    // DeviceManager,
    DeviceList,     // The main window / landing page.
    CameraControls, // Represents any number of cameras details pages (1 per camera).
}

enum DialogType {
    Debug,
    Info,
    Warn,
    Error,
}

impl DialogType {
    fn as_str(&self) -> &str {
        match self {
            DialogType::Debug => "DEBUG",
            DialogType::Info => "INFO",
            DialogType::Warn => "WARN",
            DialogType::Error => "ERROR",
        }
    }
}

// TODO: This is an example for the sake of GUI functionality.
struct CamData {
    name: String,
}

struct GUITab {
    kind: GUITabKind,
    surface: SurfaceIndex,
    node: NodeIndex,
}

struct GenCamTabsViewer {
    modal_active: bool,
    num_cameras: u32,
    // List of camera IDs

    // HashMap<UCID, CamData>
    connected_cameras: HashMap<String, CamData>,
}

impl egui_dock::TabViewer for GenCamTabsViewer {
    type Tab = String;
    
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }
    
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.set_enabled(!self.modal_active);
        
        let utid = tab.split_whitespace().last().unwrap();

        let tab_id = tab.as_str();
        ui.label(format!("This is a tab for Camera {}.", tab_id));

        ui.label(format!("This tab has ID {}", utid));

        match tab.as_str() {
            "Device List" => self.tab_device_list(ui), // Only one device list tab.
            _ => self.tab_camera_controls(ui, utid),         // Variable number of camera controls tabs.
        }
    }

    /// Unique ID for this tab.
    ///
    /// If not implemented, uses tab title text as an ID source.
    // fn id(&mut self, tab: &mut Self::Tab) -> Id {
    //     Id::new(tab.split_whitespace().last().unwrap())
    // }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    // fn on_add(&mut self, surface: SurfaceIndex, node: NodeIndex) {
    //     self.added_nodes.insert()

    //     self.added_nodes.push(GUITab {
    //         kind: GUITabKind::CameraControls,
    //         surface,
    //         node,
    //     });
    // }
}

impl GenCamTabsViewer {
    fn tab_device_list(&mut self, ui: &mut egui::Ui) {
        ui.label("This is tab 1.");

        // TODO: Replace this button with the intended connection functionality.
        if ui.button("Add Camera").on_hover_text("Add a new camera to the list.").clicked() {
            self.add_camera();
        }
        
    }
    
    fn add_camera(&mut self) {
        // Add a new camera to the list.
        self.num_cameras += 1;
        self.connected_cameras.insert(self.num_cameras.to_string(), CamData { name: "Example Camera".to_owned() });
    }
    
    fn tab_camera_controls(&mut self, ui: &mut egui::Ui, utid: &str) {
        // TODO: Handle the fact that each camera control tab will be a separate camera. Will involve using the tab ID (UTID) to look up the camera in the hashmap.

        ui.label("This is tab 2.");
        ui.label(format!("This tab is named {}", utid));
        // ui.label(format!("This tab is named {}", ));
    }
}

struct GenCamGUI {
    tabs: GenCamTabsViewer,
    tree: DockState<String>,
    ucids_tabs: Vec<String>,

    dialog_type: DialogType,
    modal_message: String,
    dark_mode: bool,

    name: String,
    age: u32,
}

impl Default for GenCamGUI {
    fn default() -> Self {
        let mut tree = DockState::new(vec!["Device List".to_owned()]); // Tabs such as "Cam 0", "Cam 1", etc will be added during runtime as necessary.

        // Example of how to add a new tab.
        // tree.push_to_focused_leaf("Camera Controls".to_owned());

        // // You can modify the tree before constructing the dock
        // let [a, b] = tree.main_surface_mut().split_right(
        //     NodeIndex::root(),
        //     0.5,
        //     vec!["Data Plot".to_owned()],
        // );
        // let [_, _] = tree
        //     .main_surface_mut()
        //     .split_below(a, 0.8, vec!["Data Log".to_owned()]);

        let tabs = GenCamTabsViewer {
            modal_active: false,
            num_cameras: 0,
            connected_cameras: HashMap::new(),
        };
        
        Self {
            tree,
            tabs,
            ucids_tabs: Vec::new(),

            dialog_type: DialogType::Debug,
            modal_message: String::new(),
            dark_mode: false,

            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl GenCamGUI {
    /// Instantiates an instance of a modal dialog window.
    fn dialog(&mut self, dialog_type: DialogType, message: &str) {
        match self.tabs.modal_active {
            true => {
                println!(
                    "A modal window is already active. The offending request was: [{}] {}",
                    dialog_type.as_str(),
                    message
                );
            }
            false => {
                self.tabs.modal_active = true;
                self.dialog_type = dialog_type;
                self.modal_message = message.to_owned();
            }
        }
    }

    /// Should be called each frame a dialog window needs to be shown.
    ///
    /// Should not be used to instantiate an instance of a dialog window, use `dialog()` instead.
    fn show_dialog(&mut self, ctx: &egui::Context) {
        self.tabs.modal_active = true;

        let title = self.dialog_type.as_str();

        egui::Window::new(title)
            .collapsible(false)
            .open(&mut self.tabs.modal_active)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let scale = 0.25;
                    match self.dialog_type {
                        DialogType::Debug => {
                            ui.add(
                                egui::Image::new(egui::include_image!(
                                    "../res/Gcg_Information.png"
                                ))
                                .fit_to_original_size(scale),
                            );
                        }
                        DialogType::Info => {
                            ui.add(
                                egui::Image::new(egui::include_image!(
                                    "../res/Gcg_Information.png"
                                ))
                                .fit_to_original_size(scale),
                            );
                        }
                        DialogType::Warn => {
                            ui.add(
                                egui::Image::new(egui::include_image!("../res/Gcg_Warning.png"))
                                    .fit_to_original_size(scale),
                            );
                        }
                        DialogType::Error => {
                            ui.add(
                                egui::Image::new(egui::include_image!("../res/Gcg_Error.png"))
                                    .fit_to_original_size(scale),
                            );
                        }
                    }
                    // ui.add(egui::Image::new(egui::include_image!(img_path)));
                    // ui.add(egui::Image::new(egui::include_image!(self.dialog_type.get_image_url())));

                    ui.vertical(|ui| {
                        // ui.add(egui::Label::new(self.modal_message.to_owned()).wrap(true));
                        ui.add(egui::Label::new(self.modal_message.to_owned()).wrap())
                    });
                });

                // if ui.button("Ok").clicked() {
                //     self.modal_active = false;
                // }
            });
    }
}

impl eframe::App for GenCamGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.5);

        //////////////////////////////////////////////////////////////
        // All possible modal window popups should be handled here. //
        //////////////////////////////////////////////////////////////

        // There should only ever be one modal window active, and it should be akin to a dialog window - info, warn, or error.

        if self.tabs.modal_active {
            self.show_dialog(ctx);
        }

        //////////////////////////////////////////////////////////////
        //////////////////////////////////////////////////////////////
        //////////////////////////////////////////////////////////////

        // Debug Controls Window for Developer Use Only
        egui::Window::new("Developer Controls").show(ctx, |ui| {
            // ui.heading("Developer Controls");
            ui.horizontal(|ui| {
                ui.label("Modal Controller:");
                if ui.button("Close").clicked() {
                    self.tabs.modal_active = false;
                }
                if ui.button("Debug").clicked() {
                    self.dialog(DialogType::Debug, "This is a debug message. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam pharetra ex quis lacus efficitur luctus. Praesent sed lectus convallis, malesuada ex nec, pulvinar tortor. Pellentesque suscipit malesuada diam, sit amet lacinia nisi maximus in. Praesent mi tortor, pulvinar et pretium sed, maximus vitae nulla. Sed vitae nibh a ligula tempus rhoncus et ac mauris. Proin ipsum eros, aliquet quis sodales ac, egestas in mi. Curabitur est metus, sollicitudin in tincidunt ut, pulvinar eget turpis. Cras nec mattis quam, non ornare ipsum. Aliquam et viverra mauris, eget semper metus. Morbi imperdiet dui est, id posuere leo luctus imperdiet. ");
                }
                if ui.button("Info").clicked() {
                    self.dialog(DialogType::Info, "This is an informational message. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam pharetra ex quis lacus efficitur luctus. Praesent sed lectus convallis, malesuada ex nec, pulvinar tortor. Pellentesque suscipit malesuada diam, sit amet lacinia nisi maximus in. Praesent mi tortor, pulvinar et pretium sed, maximus vitae nulla. Sed vitae nibh a lig");
                }
                if ui.button("Warn").clicked() {
                    self.dialog(DialogType::Warn, "This is a warning message. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam pharetra ex quis lacus efficitur luctus. Praesent sed lectus convallis, malesuada ex nec, pulvinar tortor. Pe");
                }
                if ui.button("Error").clicked() {
                    self.dialog(DialogType::Error, "This is an error message.");
                }
            });
        });

        // Top Settings Panel
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_enabled(!self.tabs.modal_active);

                ui.horizontal(|ui| {
                    menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Open").clicked() {
                                // …
                            }
                        });
                        ui.menu_button("Edit", |ui| {
                            if ui.button("Open").clicked() {
                                // …
                            }
                        });
                        ui.menu_button("View", |ui| match self.dark_mode {
                            true => {
                                if ui.button("Switch to Light Mode").clicked() {
                                    ctx.set_visuals(Visuals::light());
                                    self.dark_mode = false;
                                }
                            }
                            false => {
                                if ui.button("Switch to Dark Mode").clicked() {
                                    ctx.set_visuals(Visuals::dark());
                                    self.dark_mode = true;
                                }
                            }
                        });
                        ui.menu_button("About", |ui| {
                            if ui.button("Open").clicked() {
                                // …
                            }
                        });
                        ui.menu_button("Help", |ui| {
                            if ui.button("Open").clicked() {
                                // …
                            }
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        ui.label(format!("v{}", env!("CARGO_PKG_VERSION")))
                    });
                });
            });
    
        // New tabs are created here based on the information in the cameras hashmap.
        for (ucid, camera) in self.tabs.connected_cameras.iter() {
            // Iterate through list of camera IDs and ensure there is a tab for each.
            if !self.ucids_tabs.contains(ucid) {
                // Creates a new tab.
                self.tree.push_to_first_leaf(format!("{} {}", camera.name.clone(), ucid));
                // Adds the UCID to the list of tabs we've generated so we don't duplicate tabs.
                self.ucids_tabs.push(ucid.to_owned());
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            DockArea::new(&mut self.tree)
                // .style(Style::from_egui(ctx.style().as_ref()))
                .show(ctx, &mut self.tabs);
        });
    }
}

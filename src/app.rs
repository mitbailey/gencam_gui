#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::any::Any;
use std::time::SystemTime;
use std::vec;
use std::collections::HashMap;

use egui::load::SizedTexture;
use refimage::{DynamicImageData, GenericImage};
use image::{open, DynamicImage, ImageReader};

use eframe::egui;
use eframe::egui::{Margin, Visuals};
use egui::{menu, ImageSource};
use egui::{Frame, Widget, Id, Image};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};
use eframe::egui::load::Bytes;
use std::io::Cursor;

use core::str;
use std::io::prelude::*;
use std::net::TcpStream;

use refimage::{GenericImageOwned};
use std::path::Path;

#[derive(Debug, Clone)]
enum GUITabKind {
    // DeviceManager,
    DeviceList,     // The main window / landing page.
    CameraControls, // Represents any number of cameras details pages (1 per camera).
}

#[derive(Debug, Clone)]
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

#[derive(Clone)]
struct GUITab {
    kind: GUITabKind,
    surface: SurfaceIndex,
    node: NodeIndex,
}

// TODO: This is an example for the sake of GUI functionality.
#[derive(Debug, Clone)]
struct CamData {
    name: String,
}

#[derive(Debug)]
struct GenCamTabsViewer {
    modal_active: bool,
    num_cameras: u32,
    // List of camera IDs

    // HashMap<UCID, CamData>
    comms_stream: Option<TcpStream>,
    comms_buffer: [u8; 4096],
    server_connection: bool,
    connected_cameras: HashMap<String, CamData>,

    data: Option<Bytes>,
    uri: String,

    frame: egui::Frame,
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
    fn new(path: &str) -> Self {
        // let img = image::open(path).unwrap().to_rgb8();
        // let mut data = Cursor::new(Vec::new());
        // img.write_to(&mut data, image::ImageFormat::Png).unwrap();

        Self {
            modal_active: false,
            num_cameras: 0,

            comms_stream: None,
            comms_buffer: [0; 4096],
            server_connection: false,

            connected_cameras: HashMap::new(),

            data: None,
            uri: "image/png".into(),

            frame: egui::Frame {
                inner_margin: 6.0.into(),
                outer_margin: 3.0.into(),
                rounding: 3.0.into(),
                shadow: egui::Shadow::NONE,
                //  {
                //     offset: [2.0, 3.0].into(),
                //     blur: 16.0,
                //     spread: 0.0,
                //     color: egui::Color32::from_black_alpha(245),
                // },
                fill: egui::Color32::from_white_alpha(255),
                stroke: egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
            },
        }
    }

    fn add_camera(&mut self) {
        // Add a new camera to the list.
        self.num_cameras += 1;
        self.connected_cameras.insert(self.num_cameras.to_string(), CamData { name: format!("Example Camera #{}", self.num_cameras) });
    }

    fn connect_to_server(&mut self) -> std::io::Result<()> {
        println!("Attempting connection to server...");
        let mut stream = TcpStream::connect("127.0.0.1:50042")?;
        let mut buffer = [0; 4096];
        
        let _ = stream.read(&mut buffer[..])?;
        println!("Rxed Msg (Exp. Hello): {}", str::from_utf8(&buffer).unwrap());

        self.server_connection = true;

        self.comms_stream = Some(stream);

        Ok(())
    }

    fn receive_test_image(&mut self) -> std::io::Result<()> {
        let mut stream = self.comms_stream.as_ref().unwrap();
        let mut buffer = [0; 4096];

        // Image test transfer.
        stream.write_all(b"SEND IMAGE TEST")?;
        let _ = stream.read(&mut buffer[..])?;
        println!("Rxed Msg (Exp. SEND IMAGE TEST): {}", str::from_utf8(&buffer).unwrap());

        // RX and deserialize...
        let rimg: GenericImageOwned = serde_json::from_str(str::from_utf8(&buffer).unwrap().trim_end_matches(char::from(0))).unwrap(); // Deserialize to generic image.
        println!("{:?}", rimg.get_metadata());
        println!("{:?}", rimg.get_image());
        let img: DynamicImage = rimg.get_image().clone().try_into().expect("Could not convert image");

        let mut data = Cursor::new(Vec::new());
        img.write_to(&mut data, image::ImageFormat::Png).unwrap();
        self.data = Some(data.into_inner().into());

        Ok(())
    }

    fn tab_device_list(&mut self, ui: &mut egui::Ui) {
        ui.label("This is tab 1.");

        if ui.button("Connect to Server").clicked() {
            self.connect_to_server();
        }

        if self.server_connection {
            ui.label("Connected to server.");
        } else {
            ui.label("Not connected to server.");
        }

        // TODO: Replace this button with the intended connection functionality.
        if ui.button("Add Camera").on_hover_text("Add a new camera to the list.").clicked() {
            self.add_camera();
        }
        
    }
    
    // Camera Control tab UI.
    // BOOKMARK (UI): This is where the camera control tab UI is defined.
    fn tab_camera_controls(&mut self, ui: &mut egui::Ui, utid: &str) {
        let winsize = ui.ctx().input(|i: &egui::InputState| i.screen_rect());
        let win_width = winsize.width();
        let win_height = winsize.height();

        ui.label(format!("The window size is: {} x {}", win_width, win_height));
        
        // TODO: Handle the fact that each camera control tab will be a separate camera. Will involve using the tab ID (UTID) to look up the camera in the hashmap.
        
        ui.label(format!("This tab has Unique Tab ID {}", utid));
        ui.label(format!("{:?}", self.connected_cameras.get(utid)));

        egui::TopBottomPanel::bottom("status_panel").show(ui.ctx(), |ui| {
            ui.label(format!("Hello world from {}!", utid));
        });

        ui.columns(2, |col| {
            // When inside a layout closure within the column we can just use 'ui'.

            // FIRST COLUMN
            col[0].label("First column");
            col[0].vertical(|ui| {
                // Here we show the image data.
                self.frame.show(ui, |ui| {
                    if let Some(data) = &self.data {
                        ui.add(
                            egui::Image::new(ImageSource::Bytes { uri: self.uri.clone().into(), bytes: data.clone() })
                            .rounding(10.0)
                            .fit_to_original_size(1.0)
                        );
                    } else {
                        ui.label("No image data.");
                    }
                });
    
                self.frame.show(ui, |ui| {
                    ui.label("Image Controls");
        
                    ui.horizontal_wrapped(|ui: &mut egui::Ui| {
                        // Examples / tests on on-the-fly image manipulation.
                        // Button
                        if ui.button("Swap Image").on_hover_text("Swap the image data.").clicked() {
                            let img = image::open("res/Gcg_Warning.png").unwrap().to_rgb8();
                            let mut data = Cursor::new(Vec::new());
                            img.write_to(&mut data, image::ImageFormat::Png).unwrap();
                            self.data = Some(data.into_inner().into());
                        }
                
                        if ui.button("Reload Image").on_hover_text("Refresh the image to reflect changed data.").clicked() {
                            ui.ctx().forget_image(&self.uri.clone());
                        }
                
                        if ui.button("Nuke Image").on_hover_text("Set all bytes to 0x0.").clicked() {
                            // Change all values in self.data to 0.
                            self.data.take();
                
                            ui.ctx().forget_image(&self.uri.clone());
                        }
                    });
                });
            });
    
            // SECOND COLUMN
            col[1].label("Second column");
            col[1].vertical(|ui| {
                self.frame.show(ui, |ui| {
                    ui.collapsing("GenCam Controls 1", |ui| {
                        ui.label("This is a collapsible section.");
                        if ui.button("Acquire Image").on_hover_text("Acquire an image from the camera.").clicked() {
                            // Acquire image.
                            self.receive_test_image();
                        }
                    });
                }); 

                self.frame.show(ui, |ui| {
                    ui.collapsing("GenCam Controls 2", |ui| {
                        ui.label("This is a collapsible section.");
                    });
                }); 

                self.frame.show(ui, |ui| {
                    ui.collapsing("Non-GenCam Controls", |ui| {
                        ui.label("This is a collapsible section.");
                        if ui.button("Get Exposure").on_hover_text("On hover text TBD.").clicked() {
                            // Get exposure value.
                        }
                        if ui.button("Set Exposure").on_hover_text("On hover text TBD.").clicked() {
                            // Set exposure value.
                        }
                        ui.checkbox(&mut true, "Enable Auto-Exposure");
                    });
                }); 

                self.frame.show(ui, |ui| {
                    ui.collapsing("File Saving", |ui| {
                        ui.label("This is a collapsible section.");
                    });
                }); 
            });
        });
        
        if let Some(data) = &self.data {
            let sum: i64 = data.iter().map(|&x| x as i64).sum();
            ui.label(format!("{}", sum));
        } else {
            ui.label("No image data.");
        }

        // Image controls
        egui::Frame::default()
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .rounding(ui.visuals().widgets.noninteractive.rounding)
        .inner_margin(3.0)
        .outer_margin(3.0)
        // .shadow(egui::Shadow::new([8.0, 12.0].into(), 16.0, egui::Color32::from_black_alpha(180)))
        .show(ui, |ui| {
            ui.label("Test text!");
        });

    }
}
// #[derive(Clone)]
pub struct GenCamGUI {
    tabs: GenCamTabsViewer,
    tree: DockState<String>,
    // tree_cc: DockState<String>, // Tree for Camera Controls tabs.
    // tree_dl: DockState<String>, // Tree for Device List tabs.
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
        // let mut tree_dl = DockState::new(vec!["Device List".to_owned()]); // Tabs such as "Cam 0", "Cam 1", etc will be added during runtime as necessary.

        // let mut tree_cc = DockState::new(vec!["Camera Controls".to_owned()]);
        // tree_cc.push_to_focused_leaf("Camera Controls".to_owned());
        // let [a, b] = tree.main_surface_mut().split_below(
        //     NodeIndex::root(),
        //     0.5,
        //     vec!["Data Plot".to_owned()],
        // );
        // let [_, _] = tree_cc
        //     .main_surface_mut()
        //     .split_right(a, 0.8, vec!["Data Log".to_owned()]);

        // Example of how to add a new tab.
        // tree.push_to_focused_leaf("Camera Controls".to_owned());

        // You can modify the tree before constructing the dock
        // let [a, b] = tree.main_surface_mut().split_right(
        //     NodeIndex::root(),
        //     0.5,
        //     vec!["Data Plot".to_owned()],
        // );
        // let [_, _] = tree
        //     .main_surface_mut()
        //     .split_below(a, 0.8, vec!["Data Log".to_owned()]);


        let tabs = GenCamTabsViewer::new("res/Gcg_Information.png");

        // let tabs = GenCamTabsViewer {
        //     modal_active: false,
        //     num_cameras: 0,
        //     connected_cameras: HashMap::new(),
        //     // image: GenericImage::new(SystemTime::now(), img),
        //     texture: None,
        // };
        
        Self {
            tree,
            // tree_cc,
            // tree_dl,
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
                                    // ctx.set_visuals_of(egui::Theme::Light, Visuals::light());
                                    self.dark_mode = false;
                                }
                            }
                            false => {
                                if ui.button("Switch to Dark Mode").clicked() {
                                    ctx.set_visuals(Visuals::dark());
                                    // ctx.set_visuals_of(egui::Theme::Dark, Visuals::dark());
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

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
use egui::Ui;

use core::str;
use std::io::prelude::*;
use std::net::TcpStream;

use refimage::{GenericImageOwned};
use std::path::Path;
use circular_buffer::CircularBuffer;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};


struct WsBackend {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    events: Vec<WsEvent>,
    message: String,
}

impl WsBackend {
    fn connect(uri: &str, ctx: &Option<egui::Context>) -> Option<WsBackend> {
        let res = if let Some(ctx) = ctx {
            let ctx = ctx.clone();
            let wakeup = move || ctx.request_repaint();
            ewebsock::connect_with_wakeup(uri, Default::default(), wakeup)
        } else {
            ewebsock::connect(uri, Default::default())
        };
        match res {
            Ok((ws_sender, ws_receiver)) => {
                let ws = WsBackend {
                    ws_sender,
                    ws_receiver,
                    events: Vec::new(),
                    message: String::new(),
                };
                Some(ws)
            }
            Err(e) => {
                eprintln!("Failed to connect to websocket: {}", e);
                None
            }
        }
    }

    fn close(&mut self) {
        self.ws_sender.close();
    }

    fn ui(&mut self, ui: &mut Ui) {
        while let Some(event) = self.ws_receiver.try_recv() {
            self.events.push(event);
        }
        
        ui.horizontal(|ui| {
            ui.label("Message to send:");
            if ui.text_edit_singleline(&mut self.message).lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                self.ws_sender
                    .send(WsMessage::Text(std::mem::take(&mut self.message)));
            }
        });

        ui.separator();
        ui.heading("Received events:");
        for event in &self.events {
            ui.label(format!("{event:?}"));
        }
    }
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

// TODO: This is an example for the sake of GUI functionality.
#[derive(Debug, Clone)]
struct CamData {
    name: String,
}

// #[derive(Clone)]
pub struct GenCamGUI {
    // tabs: GenCamTabsViewer,
    // tree: DockState<String>,
    // tree_cc: DockState<String>, // Tree for Camera Controls tabs.
    // tree_dl: DockState<String>, // Tree for Device List tabs.
    // ucids_tabs: Vec<String>,

    dialog_type: DialogType,
    modal_message: String,
    dark_mode: bool,

    modal_active: bool,
    // num_cameras: u32,
    // List of camera IDs

    // HashMap<UCID, CamData>
    comms_stream: Option<TcpStream>,
    comms_buffer: [u8; 4096],
    server_connection: bool,
    connected_cameras: HashMap<String, CamData>,

    data: Option<Bytes>,
    img_uri: String,

    frame: egui::Frame,

    msg_list: CircularBuffer<150, String>,

    // Websocket
    pub uri: String,
    pub ws: Option<WsBackend>,
    pub ctx: Option<egui::Context>,
}

impl Default for GenCamGUI {
    fn default() -> Self {
        
        Self {

            dialog_type: DialogType::Debug,
            modal_message: String::new(),
            dark_mode: false,

            modal_active: false,

            comms_stream: None,
            comms_buffer: [0; 4096],
            server_connection: false,

            connected_cameras: HashMap::new(),

            data: None,
            img_uri: "image/png".into(),

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

            msg_list: CircularBuffer::new(),
            uri: "ws://localhost:9001".into(),
            ws: None,
            ctx: None,
        }
    }
}

impl GenCamGUI {    
    /// Instantiates an instance of a modal dialog window.
    fn dialog(&mut self, dialog_type: DialogType, message: &str) {
        match self.modal_active {
            true => {
                println!(
                    "A modal window is already active. The offending request was: [{}] {}",
                    dialog_type.as_str(),
                    message
                );
            }
            false => {
                self.modal_active = true;
                self.dialog_type = dialog_type;
                self.modal_message = message.to_owned();
            }
        }
    }

    /// Should be called each frame a dialog window needs to be shown.
    ///
    /// Should not be used to instantiate an instance of a dialog window, use `dialog()` instead.
    fn show_dialog(&mut self, ctx: &egui::Context) {
        self.modal_active = true;

        let title = self.dialog_type.as_str();

        egui::Window::new(title)
            .collapsible(false)
            .open(&mut self.modal_active)
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

    fn connect_to_server(&mut self) -> std::io::Result<()> {
        println!("Attempting connection to server...");
        let mut stream = TcpStream::connect("127.0.0.1:50042")?;
        let mut buffer = [0; 4096];

        let _ = stream.read(&mut buffer[..])?;
        println!(
            "Rxed Msg (Exp. Hello): {}",
            str::from_utf8(&buffer).unwrap()
        );

        self.server_connection = true;

        self.comms_stream = Some(stream);

        Ok(())
    }

    fn receive_test_image(&mut self) -> std::io::Result<()> {
        self.msg_list.push_back("Attempting to receive image...".to_owned());
        let mut stream = self.comms_stream.as_ref().unwrap();
        let mut buffer = [0; 4096];
        
        self.msg_list.push_back("Writing to server.".to_owned());
        // Image test transfer.
        stream.write_all(b"SEND IMAGE TEST")?;
        let _ = stream.read(&mut buffer[..])?;
        println!(
            "Rxed Msg (Exp. SEND IMAGE TEST): {}",
            str::from_utf8(&buffer).unwrap()
        );

        // RX and deserialize...
        let rimg: GenericImageOwned = serde_json::from_str(
            str::from_utf8(&buffer)
                .unwrap()
                .trim_end_matches(char::from(0)),
        )
        .unwrap(); // Deserialize to generic image.
        println!("{:?}", rimg.get_metadata());
        println!("{:?}", rimg.get_image());
        let img: DynamicImage = rimg
            .get_image()
            .clone()
            .try_into()
            .expect("Could not convert image");

        let mut data = Cursor::new(Vec::new());
        img.write_to(&mut data, image::ImageFormat::Png).unwrap();
        self.data = Some(data.into_inner().into());

        Ok(())
    }
    
    fn ui_developer_controls(&mut self, ctx: &egui::Context) {
        // Debug Controls Window for Developer Use Only
        egui::Window::new("Developer Controls").show(ctx, |ui| {
            // ui.heading("Developer Controls");
            ui.horizontal(|ui| {
                ui.label("Modal Controller:");
                if ui.button("Close").clicked() {
                    self.modal_active = false;
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

            ui.horizontal(|ui| {
                ui.label("Server Connection:");
                let mut disconnect = false;
                if let Some(ws) = &mut self.ws {
                    ui.horizontal(|ui| {
                        ui.label("WebSocket URI: ");
                        ui.label(&self.uri);
                        if ui.button("Disconnect").clicked() {
                            disconnect = true;
                        }
                    });
                    ws.ui(ui);
                } else {
                    ui.horizontal(|ui| {
                        ui.label("WebSocket URI: ");
                        ui.text_edit_singleline(&mut self.uri);
                        if ui.button("Connect").clicked() {
                            self.ws = WsBackend::connect(&self.uri, &self.ctx);
                        }
                    }); 
                }
                if disconnect {
                    self.ws.as_mut().unwrap().close(); // safe to unwrap since disconnect is only true if ws is Some
                    self.ws = None;
                }
            });
        });
    }


    fn ui_top_bar(&mut self, ctx: &egui::Context) {
        // Top Settings Panel
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_enabled(!self.modal_active);

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
    }

    fn ui_left_panel(&mut self, ctx: &egui::Context, w_view: f32) {
        // Left Panel
        let w_scale = 0.5;

        egui::SidePanel::left("left_panel")
            .resizable(true)
            // .min_width(ctx.available_rect().width()/8.0)
            // .max_width(ctx.available_rect().width()/4.0)
            .width_range((w_view / (8.0 / w_scale))..=(w_view / (4.0 / w_scale)))
            .default_width(ctx.available_rect().width() / (6.0 / w_scale))
            .show(ctx, |ui| {
                ui.set_enabled(!self.modal_active);
                ui.label("Window Controls");
                ui.separator(); // Placeholder to enable dragging (expands to fill).

                ui.label(format!(
                    "{:?}",
                    (w_view / (8.0 / w_scale))..=(w_view / (4.0 / w_scale))
                ));
                ui.label("Left Panel");
            });
        }
        
        fn ui_right_panel(&mut self, ctx: &egui::Context, w_view: f32) {
            // Left Panel
            let w_scale = 1.0;
            
            egui::SidePanel::right("right_panel")
            .resizable(true)
            .width_range(w_view / (8.0 / w_scale)..=w_view / (2.0 / w_scale))
            .default_width(ctx.available_rect().width() / (6.0 / w_scale))
            .show(ctx, |ui| {
                ui.set_enabled(!self.modal_active);
                ui.label("Communication Log");
                ui.separator(); // Placeholder to enable dragging (expands to fill).

                if ui.button("Generate New Msg").clicked() {
                    self.msg_list
                        .push_back(format!("Hello! This is message #{}. This is a long message because it contains a lot of data!", self.msg_list.len()));
                }

                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                
                        ui.label(format!(
                            "{:?}",
                            (w_view / (8.0 / w_scale))..=(w_view / (4.0 / w_scale))
                        ));
                        ui.label("Right Panel");
                        for msg in self.msg_list.iter() {
                            ui.add(egui::Label::new(msg).truncate());
                        }
                });
            });
    }

    fn ui_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Test.");
            ui.label(format!("Avail   {:?}", ctx.available_rect()));
            ui.label(format!("Used    {:?}", ctx.used_rect()));
            ui.label(format!("Screen  {:?}", ctx.screen_rect()));

            let winsize = ui.ctx().input(|i: &egui::InputState| i.screen_rect());
            let win_width = winsize.width();
            let win_height = winsize.height();

            ui.label(format!(
                "The window size is: {} x {}",
                win_width, win_height
            ));

            // TODO: Handle the fact that each camera control tab will be a separate camera. Will involve using the tab ID (UTID) to look up the camera in the hashmap.

            // ui.label(format!("This tab has Unique Tab ID {}", utid));
            // ui.label(format!("{:?}", self.connected_cameras.get(utid)));

            // egui::TopBottomPanel::bottom("status_panel").show(ui.ctx(), |ui| {
            //     ui.label(format!("Hello world from {}!", utid));
            // });

            ui.columns(2, |col| {
                // When inside a layout closure within the column we can just use 'ui'.

                // FIRST COLUMN
                col[0].label("First column");
                col[0].vertical(|ui| {
                    // Here we show the image data.
                    self.frame.show(ui, |ui| {
                        if let Some(data) = &self.data {
                            ui.add(
                                egui::Image::new(ImageSource::Bytes {
                                    uri: self.img_uri.clone().into(),
                                    bytes: data.clone(),
                                })
                                .rounding(10.0)
                                .fit_to_original_size(1.0),
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
                            if ui
                                .button("Swap Image")
                                .on_hover_text("Swap the image data.")
                                .clicked()
                            {
                                let img = image::open("res/Gcg_Warning.png").unwrap().to_rgb8();
                                let mut data = Cursor::new(Vec::new());
                                img.write_to(&mut data, image::ImageFormat::Png).unwrap();
                                self.data = Some(data.into_inner().into());
                            }

                            if ui
                                .button("Reload Image")
                                .on_hover_text("Refresh the image to reflect changed data.")
                                .clicked()
                            {
                                ui.ctx().forget_image(&self.img_uri.clone());
                            }

                            if ui
                                .button("Nuke Image")
                                .on_hover_text("Set all bytes to 0x0.")
                                .clicked()
                            {
                                // Change all values in self.data to 0.
                                self.data.take();

                                ui.ctx().forget_image(&self.img_uri.clone());
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
                            if ui
                                .button("Acquire Image")
                                .on_hover_text("Acquire an image from the camera.")
                                .clicked()
                            {
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
                            if ui
                                .button("Get Exposure")
                                .on_hover_text("On hover text TBD.")
                                .clicked()
                            {
                                // Get exposure value.
                            }
                            if ui
                                .button("Set Exposure")
                                .on_hover_text("On hover text TBD.")
                                .clicked()
                            {
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
        });
    }

    fn ui_bottom_bar(&mut self, ctx: &egui::Context) {
        // Bottom Status Panel
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_enabled(!self.modal_active);

                ui.horizontal(|ui| {
                    ui.label("Bottom Status Panel");
                });
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

        if self.modal_active {
            self.show_dialog(ctx);
        }

        //////////////////////////////////////////////////////////////
        //////////////////////////////////////////////////////////////
        //////////////////////////////////////////////////////////////

        let w_view = ctx.screen_rect().width();

        self.ui_developer_controls(ctx);
        self.ui_top_bar(ctx);
        self.ui_left_panel(ctx, w_view);
        self.ui_right_panel(ctx, w_view);
        self.ui_bottom_bar(ctx);
        self.ui_central_panel(ctx);
    }
}

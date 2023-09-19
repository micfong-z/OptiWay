use std::{f32::consts::PI, fmt::Display, path::Path};

use egui::{emath, pos2, Color32, Rect};
use rfd::FileDialog;

use crate::{md_icons, setup_custom_fonts, setup_custom_styles};

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Algorithm {
    #[default]
    First,
    Second,
    Third,
}

impl Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Algorithm::First => write!(f, "First"),
            Algorithm::Second => write!(f, "Second"),
            Algorithm::Third => write!(f, "Third"),
        }
    }
}

pub struct OptiWayApp {
    selected_student: Option<String>,
    selected_algorithm: Algorithm,
    selected_period: usize,
    selected_day: u32,
    selected_floor: [bool; 9],
    selected_floor_index: usize,
    textures: Vec<Option<egui::TextureHandle>>,
    inactive_brightness: u8,
}

impl Default for OptiWayApp {
    fn default() -> Self {
        let mut floors = [false; 9];
        floors[0] = true;
        Self {
            selected_student: Default::default(),
            selected_algorithm: Default::default(),
            selected_period: Default::default(),
            selected_day: 1,
            selected_floor: floors,
            selected_floor_index: 0,
            textures: vec![None; 9],
            inactive_brightness: 64,
        }
    }
}

impl OptiWayApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        setup_custom_styles(&cc.egui_ctx);

        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        Default::default()
    }
}

impl eframe::App for OptiWayApp {
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label("OptiWay");
                ui.separator();
                ui.label(md_icons::material_design_icons::MDI_CALENDAR_ALERT)
                    .on_hover_text("Timetable not imported.");
                ui.separator();
                ui.label(md_icons::material_design_icons::MDI_ACCOUNT_ALERT)
                    .on_hover_text("No student selected.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Ready")
                });
            });
        });

        egui::SidePanel::right("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.heading("Data source");
                    if ui.button("Import timetable").clicked() {
                        FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_directory("/")
                            .pick_file();
                    }
                    egui::ComboBox::from_label("Algorithm")
                        .selected_text(self.selected_algorithm.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_algorithm,
                                Algorithm::First,
                                "First",
                            );
                            ui.selectable_value(
                                &mut self.selected_algorithm,
                                Algorithm::Second,
                                "Second",
                            );
                            ui.selectable_value(
                                &mut self.selected_algorithm,
                                Algorithm::Third,
                                "Third",
                            );
                        });
                    if ui.button("OptiWay!").clicked() {
                        todo!();
                    }
                    ui.separator();
                    egui::ComboBox::from_label("Student")
                        .selected_text(self.selected_student.clone().unwrap_or("—".to_owned()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_student,
                                Some("First".to_owned()),
                                "First",
                            );
                            ui.selectable_value(
                                &mut self.selected_student,
                                Some("Second".to_owned()),
                                "Second",
                            );
                            ui.selectable_value(
                                &mut self.selected_student,
                                Some("Third".to_owned()),
                                "Third",
                            );
                        });
                    egui::ComboBox::from_label("Day of Week")
                        .selected_text(self.selected_day.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.selected_day, 1, "Monday");
                            ui.selectable_value(&mut self.selected_day, 2, "Tuesday");
                            ui.selectable_value(&mut self.selected_day, 3, "Wednesday");
                        });
                    egui::ComboBox::from_label("Period")
                        .selected_text(self.selected_period.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.selected_period, 0, "MR");
                            ui.selectable_value(&mut self.selected_period, 1, "MR-P1");
                            ui.selectable_value(&mut self.selected_period, 2, "P1-P2");
                        });
                    ui.separator();
                    ui.heading("Path");
                    if ui.button("Export path (all students)").clicked() {
                        todo!();
                    }
                    if ui.button("Export path (current student)").clicked() {
                        todo!();
                    }
                    if ui.button("Show path as text").clicked() {
                        todo!();
                    }
                    ui.separator();
                    ui.heading("Floor view");
                    ui.horizontal(|ui| {
                        if ui
                            .toggle_value(&mut self.selected_floor[0], "All")
                            .clicked()
                        {
                            self.selected_floor_index = 0;
                            for i in 1..=8 {
                                self.selected_floor[i] = false;
                            }
                            if !self.selected_floor.contains(&true) {
                                self.selected_floor[0] = true;
                            }
                        };
                        for i in 2..=8 {
                            if ui
                                .toggle_value(&mut self.selected_floor[i], format!("{}F", i))
                                .clicked()
                            {
                                self.selected_floor_index = i;
                                for j in 0..=8 {
                                    if i != j {
                                        self.selected_floor[j] = false;
                                    }
                                }
                                if !self.selected_floor.contains(&true) {
                                    self.selected_floor[i] = true;
                                }
                            }
                        }
                    });
                    ui.add(
                        egui::Slider::new(&mut self.inactive_brightness, 32..=255)
                            .text("Inactive floor brightness"),
                    );
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Import textures if uninitialized

            let mut textures: Vec<egui::TextureHandle> = Vec::new();
            for i in 2..=8 {
                let texture_cur: &egui::TextureHandle = self.textures[i].get_or_insert_with(|| {
                    ui.ctx().load_texture(
                        format!("texture-floor-projection-{i}F"),
                        load_image_from_path(Path::new(
                            format!("../assets/projection-transparent/projection_{i}F.png")
                                .as_str(),
                        ))
                        .unwrap(),
                        Default::default(),
                    )
                });
                textures.push(texture_cur.clone());
            }
            ui.horizontal(|ui| {
                ui.heading("OptiWay");
                egui::warn_if_debug_build(ui);
            });

            let desired_size = ui.available_size_before_wrap();
            if desired_size.y < desired_size.x / 2243.0 * (1221.0 + 350.0) {
                ui.label("▲ There may not be enough space to display the floor plan.");
            }
            let (_id, rect) = ui.allocate_space(desired_size);
            let scale = rect.width() / 2243.0;
            let mut bottom_right_x = 0.0;

            // Paint floor projections

            for (i, texture) in textures.iter().enumerate().take(7) {
                let rect = Rect::from_min_size(
                    rect.min,
                    emath::vec2(rect.width(), rect.width() / texture.aspect_ratio()),
                )
                .translate(emath::vec2(0.0, (7 - i) as f32 * 50.0 * scale));

                if bottom_right_x == 0.0 {
                    bottom_right_x = rect.max.x;
                }

                ui.painter().image(
                    texture.into(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    if self.selected_floor[0] || self.selected_floor[i + 2] {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(self.inactive_brightness)
                    },
                );
            }
            // Special case: the floor is selected, so needs to be repainted last
            if self.selected_floor_index != 0 {
                let texture = textures[self.selected_floor_index - 2].clone();
                let rect = Rect::from_min_size(
                    rect.min,
                    emath::vec2(rect.width(), rect.width() / texture.aspect_ratio()),
                )
                .translate(emath::vec2(
                    0.0,
                    (7 - (self.selected_floor_index - 2)) as f32 * 50.0 * scale,
                ));

                ui.painter().image(
                    (&texture).into(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            ui.painter().circle_filled(
                get_projection_pos(&rect, 1325, 200, 250, scale),
                3.0,
                Color32::RED,
            );
        });
    }
}

fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}

/// Converts 3D coordinates in projection-coords.yaml to 2D coordinates on screen.
fn get_projection_pos(rect: &Rect, x: i32, y: i32, z: i32, scale: f32) -> emath::Pos2 {
    /// Projection angle of the floor plan (radians)
    const ANGLE: f32 = PI / 6.0;

    (emath::vec2(
        rect.left() + 25.0 * ANGLE.cos() * scale,
        rect.top() + (50.0 + 350.0 + 25.0 * ANGLE.sin()) * scale,
    ) + emath::vec2(
        ((x as f32) * ANGLE.cos() + (y as f32) * ANGLE.cos()) * scale,
        ((x as f32) * ANGLE.sin() - (y as f32) * ANGLE.sin() - (z as f32)) * scale,
    ))
    .to_pos2()
}

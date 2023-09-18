use std::fmt::Display;

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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OptiWay");
                egui::warn_if_debug_build(ui);
            });
        });

        egui::SidePanel::right("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.heading("Data source");
                    if ui.button("Import timetable").clicked() {
                        todo!();
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
                });
            });
        });
    }
}

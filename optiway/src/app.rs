use std::{
    collections::HashMap,
    f32::consts::PI,
    fmt::Display,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use egui::{
    color_picker, emath, pos2, CentralPanel, Color32, ColorImage, ComboBox, Grid, Layout,
    ProgressBar, Rect, RichText, Slider, Stroke, TextureHandle, Window,
};
use rfd::FileDialog;

use crate::{md_icons::material_design_icons, setup_custom_fonts, setup_custom_styles};

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Algorithm {
    #[default]
    Shortest,
}

impl Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Algorithm::Shortest => write!(f, "Minimize distance"),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
enum TimetableValidationStatus {
    #[default]
    Ready,
    Validating(i32, String),
    Failed(String),
    Successful,
}

impl TimetableValidationStatus {
    fn get_error_message(&self) -> String {
        match self {
            TimetableValidationStatus::Failed(message) => message.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Default)]
struct TimetableFileInfo {
    filename: String,
    filepath: PathBuf,
    student_count: Arc<Mutex<Option<i32>>>,
    session_count: Arc<Mutex<Option<i32>>>,
    validation_status: Arc<Mutex<TimetableValidationStatus>>,
}

pub struct OptiWayApp {
    selected_student: Option<String>,
    student_list: Arc<Mutex<Vec<String>>>,
    selected_algorithm: Algorithm,
    selected_period: usize,
    selected_day: u32,
    selected_floor: [bool; 9],
    selected_floor_index: usize,
    textures: Vec<Option<TextureHandle>>,
    inactive_brightness: u8,
    projection_coords: HashMap<String, [i32; 3]>,
    active_path_color: Color32,
    inactive_path_color: Color32,
    show_path_window: bool,
    show_json_validation: bool,
    timetable_file_info: TimetableFileInfo,
    student_number_search: String,
}

impl Default for OptiWayApp {
    fn default() -> Self {
        let mut floors = [false; 9];
        floors[0] = true;
        Self {
            selected_student: Default::default(),
            student_list: Default::default(),
            selected_algorithm: Default::default(),
            selected_period: 1,
            selected_day: 1,
            selected_floor: floors,
            selected_floor_index: 0,
            textures: vec![None; 9],
            inactive_brightness: 64,
            projection_coords: serde_yaml::from_str(
                std::fs::read_to_string("../assets/projection-coords-flatten.yaml")
                    .expect("Failed to read projection-coords-flatten.yaml")
                    .as_str(),
            )
            .unwrap(),
            active_path_color: Color32::from_rgb(0xec, 0x6f, 0x27),
            inactive_path_color: Color32::from_gray(0x61),
            show_path_window: false,
            show_json_validation: false,
            timetable_file_info: Default::default(),
            student_number_search: Default::default(),
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
                ui.label(material_design_icons::MDI_CALENDAR_ALERT)
                    .on_hover_text("Timetable not imported.");
                ui.separator();
                ui.label(material_design_icons::MDI_ACCOUNT_ALERT)
                    .on_hover_text("No student selected.");
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.show_json_validation {
                        ui.label("Validating timetable file");
                    } else {
                        ui.label("Ready");
                    }
                });
            });
        });

        egui::SidePanel::right("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.heading("Data source");
                    if ui.button("Import timetable").clicked() {
                        let file = FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_directory("../")
                            .pick_file();
                        if let Some(file) = file {
                            self.timetable_file_info.filename =
                                file.file_name().unwrap().to_str().unwrap().to_owned();
                            self.timetable_file_info.filepath = file;
                            self.show_json_validation = true;
                            *self.timetable_file_info.validation_status.lock().unwrap() =
                                TimetableValidationStatus::Ready;
                        }
                    }
                    ComboBox::from_label("Algorithm")
                        .selected_text(self.selected_algorithm.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_algorithm,
                                Algorithm::Shortest,
                                "Minimize distance",
                            );
                        });
                    if ui.button("OptiWay!").clicked() {
                        todo!();
                    }
                    ui.separator();
                    ComboBox::from_label("Student")
                        .selected_text(self.selected_student.clone().unwrap_or("—".to_owned()))
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.student_number_search)
                                    .hint_text("Search"),
                            );
                            ui.separator();
                            for student in self.student_list.lock().unwrap().iter() {
                                if student.contains(&self.student_number_search) {
                                    ui.selectable_value(
                                        &mut self.selected_student,
                                        Some(student.to_owned()),
                                        student,
                                    );
                                }
                            }
                        });
                    ComboBox::from_label("Day of Week")
                        .selected_text(convert_day_of_week(self.selected_day))
                        .show_ui(ui, |ui| {
                            for i in 1..=5 {
                                ui.selectable_value(
                                    &mut self.selected_day,
                                    i,
                                    convert_day_of_week(i),
                                );
                            }
                        });
                    ComboBox::from_label("Period")
                        .selected_text(convert_periods(self.selected_period))
                        .show_ui(ui, |ui| {
                            for i in 1..=12 {
                                ui.selectable_value(
                                    &mut self.selected_period,
                                    i,
                                    convert_periods(i),
                                );
                            }
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
                        self.show_path_window = true;
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
                        Slider::new(&mut self.inactive_brightness, 32..=255)
                            .text("Inactive floor brightness"),
                    );
                    ui.collapsing("Path color", |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("Active path color");
                                if ui.button("Reset").clicked() {
                                    self.active_path_color = Color32::from_rgb(0xec, 0x6f, 0x27);
                                }
                            });
                            color_picker::color_picker_color32(
                                ui,
                                &mut self.active_path_color,
                                color_picker::Alpha::Opaque,
                            );
                        });
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("Inactive path color");
                                if ui.button("Reset").clicked() {
                                    self.inactive_path_color = Color32::from_gray(0x61);
                                }
                            });
                            color_picker::color_picker_color32(
                                ui,
                                &mut self.inactive_path_color,
                                color_picker::Alpha::Opaque,
                            );
                        });
                    });
                });
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            // JSON validation window
            if self.show_json_validation {
                Window::new("Timetable Validation")
                    .show(ctx, |ui| {
                        ui.heading("Metadata");
                        Grid::new("timetable_validation_grid")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Filename");
                                ui.label(format!("{}", self.timetable_file_info.filename));
                                ui.end_row();

                                ui.label("Student count");
                                if let Some(student_count) = *self.timetable_file_info.student_count.lock().unwrap() {
                                    ui.label(format!("{}", student_count));
                                } else {
                                    ui.label("—");
                                }
                                ui.end_row();

                                ui.label("Session count");
                                if let Some(session_count) = *self.timetable_file_info.session_count.lock().unwrap() {
                                    ui.label(format!("{}", session_count));
                                } else {
                                    ui.label("—");
                                }
                                ui.end_row();
                            });
                        ui.separator();
                        let validation_status = self
                            .timetable_file_info
                            .validation_status
                            .lock()
                            .unwrap()
                            .clone();
                        match validation_status {
                            TimetableValidationStatus::Ready => {
                                *self
                                    .timetable_file_info
                                    .validation_status
                                    .clone().lock().unwrap() =
                                    TimetableValidationStatus::Validating(0, "Ready to validate".to_owned());
                                let filepath = self.timetable_file_info.filepath.clone();
                                let projection_coords = self.projection_coords.clone();
                                let validation_status_arc = self
                                    .timetable_file_info
                                    .validation_status
                                    .clone();
                                let student_count_arc = self
                                    .timetable_file_info
                                    .student_count
                                    .clone();
                                let session_count_arc = self
                                    .timetable_file_info
                                    .session_count
                                    .clone();
                                let student_list_arc = self.student_list.clone();
                                thread::spawn(move || {
                                    let mut rooms: Vec<String> = projection_coords.keys().map(|k| k.to_owned()).collect();
                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Validating(0, "Validating student numbers...".to_owned());
                                    rooms.push("G".into());
                                    let mut timetable_file = std::fs::File::open(filepath).unwrap();
                                    let mut timetable_json = String::new();
                                    if timetable_file
                                        .read_to_string(&mut timetable_json).is_err() {
                                        *validation_status_arc.lock().unwrap() =
                                            TimetableValidationStatus::Failed(
                                                "Failed to read timetable file"
                                                    .to_owned(),
                                            );
                                        return;
                                    };
                                    let timetable: serde_json::Value =
                                        match serde_json::from_str(&timetable_json) {
                                            Ok(t) => t,
                                            Err(e) => {
                                                *validation_status_arc.lock().unwrap() =
                                                    TimetableValidationStatus::Failed(
                                                        format!("Invalid JSON format in timetable file: {}", e)
                                                    );
                                                return;
                                            }
                                        };
                                    let mut student_count = 0;

                                    if timetable.as_object().is_none() {
                                        *validation_status_arc.lock().unwrap() =
                                            TimetableValidationStatus::Failed(
                                                "Invalid timetable file format: the JSON file is not a map"
                                                    .to_owned(),
                                            );
                                        return;
                                    }
                                    for student_key in timetable.as_object().unwrap().keys() {
                                        if student_key.chars().all(char::is_numeric) && 4 <= student_key.len() && student_key.len() <= 5 {
                                            student_count += 1;
                                            *student_count_arc.lock().unwrap() = Some(student_count);
                                        } else {
                                            *validation_status_arc.lock().unwrap() =
                                                TimetableValidationStatus::Failed(
                                                    format!(
                                                        "Invalid student number: \"{}\"",
                                                        student_key
                                                    ),
                                                );
                                            return;
                                        }
                                    }

                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Validating(10, "Validating days of the week...".to_owned());
                                    for (student_key, week_timetable) in timetable.as_object().unwrap() {
                                        if week_timetable.as_object().is_none() {
                                            *validation_status_arc.lock().unwrap() =
                                                TimetableValidationStatus::Failed(
                                                    format!("Invalid timetable file format: student {}'s timetable is not a map", student_key)
                                                );
                                            return;
                                        }
                                        let mut days_of_week = [false; 5];
                                        for day_key in week_timetable.as_object().unwrap().keys() {
                                            if day_key.chars().all(char::is_numeric) && day_key.parse::<i32>().unwrap() <= 5 && day_key.parse::<i32>().unwrap() >= 1 {
                                                days_of_week[day_key.parse::<i32>().unwrap() as usize - 1] = true;
                                            } else {
                                                *validation_status_arc.lock().unwrap() =
                                                    TimetableValidationStatus::Failed(
                                                        format!(
                                                            "Student {} has an invalid day of week: \"{}\"",
                                                            student_key, day_key
                                                        ),
                                                    );
                                                return;
                                            }
                                        }
                                        if days_of_week.contains(&false) {
                                            *validation_status_arc.lock().unwrap() =
                                                TimetableValidationStatus::Failed(format!(
                                                    "Student {} has an incomplete timetable: missing day {}",
                                                    student_key, days_of_week.iter().enumerate().filter(|(_, &b)| !b).map(|(i, _)| i + 1).collect::<Vec<usize>>().iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
                                                ), );
                                            return;
                                        }
                                    }

                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Validating(20, "Validating periods...".to_owned());
                                    for (student_key, week_timetable) in timetable.as_object().unwrap() {
                                        for (day_key, day_timetable) in week_timetable.as_object().unwrap() {
                                            if day_timetable.as_object().is_none() {
                                                *validation_status_arc.lock().unwrap() =
                                                    TimetableValidationStatus::Failed(
                                                        format!("Invalid timetable file format: student {}'s timetable on day {} is not a map", student_key, day_key)
                                                    );
                                                return;
                                            }
                                            let mut periods = [false; 10];
                                            for period_key in day_timetable.as_object().unwrap().keys() {
                                                if period_key.chars().all(char::is_numeric) && period_key.parse::<i32>().unwrap() <= 10 && period_key.parse::<i32>().unwrap() >= 1 {
                                                    periods[period_key.parse::<i32>().unwrap() as usize - 1] = true;
                                                } else {
                                                    *validation_status_arc.lock().unwrap() =
                                                        TimetableValidationStatus::Failed(
                                                            format!(
                                                                "Student {} has an invalid period on day {}: \"{}\"",
                                                                student_key, day_key, period_key
                                                            ),
                                                        );
                                                    return;
                                                }
                                            }
                                            if periods.contains(&false) {
                                                *validation_status_arc.lock().unwrap() =
                                                    TimetableValidationStatus::Failed(
                                                        format!(
                                                            "Student {} has an incomplete timetable on day {}: missing periods {}",
                                                            student_key, day_key, periods.iter().
                                                                enumerate().filter(|(_, &b)| !b).map(|(i, _)| i + 1).collect::<Vec<usize>>().iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
                                                        ),
                                                    );
                                                return;
                                            }
                                        }
                                    }

                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Validating(20, "Validating classrooms...".to_owned());
                                    let mut sessions = 0;
                                    for (student_key, week_timetable) in timetable.as_object().unwrap() {
                                        for (day_key, day_timetable) in week_timetable.as_object().unwrap() {
                                            for (period_key, room) in day_timetable.as_object().unwrap() {
                                                if !rooms.contains(&room.as_str().unwrap().to_owned()) {
                                                    *validation_status_arc.lock().unwrap() =
                                                        TimetableValidationStatus::Failed(
                                                            format!(
                                                                "Student {} has an invalid classroom on day {} period {}: {}",
                                                                student_key, day_key, period_key, room
                                                            ),
                                                        );
                                                    return;
                                                }
                                                sessions += 1;
                                                if sessions % 1000 == 0 {
                                                    *session_count_arc.lock().unwrap() = Some(sessions);
                                                    *validation_status_arc.lock().unwrap() =
                                                        TimetableValidationStatus::Validating(20 + (sessions as f32 / (student_count * 10 * 5) as f32 * 80.0) as i32, "Validating classrooms...".to_owned());
                                                }
                                            }
                                        }
                                    }

                                    *student_list_arc.lock().unwrap() = timetable.as_object().unwrap().keys().map(|k| k.to_owned()).collect();

                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Successful;
                                });
                            }
                            TimetableValidationStatus::Validating(progress, message) => {
                                ui.with_layout(
                                    Layout::top_down_justified(egui::Align::Center),
                                    |ui| {
                                        ui.label(RichText::new(
                                            material_design_icons::MDI_CALENDAR_SEARCH,
                                        ).size(32.0));
                                        ui.label(message);
                                        ui.add(ProgressBar::new(progress as f32 / 100.0));
                                    },
                                );
                            }
                            TimetableValidationStatus::Failed(_) => {
                                ui.with_layout(
                                    Layout::top_down_justified(egui::Align::Center),
                                    |ui| {
                                        ui.label(RichText::new(
                                            material_design_icons::MDI_CALENDAR_REMOVE,
                                        ).size(32.0).color(Color32::from_rgb(0xe4, 0x37, 0x48)));
                                        ui.label("Validation failed");
                                        ui.label(validation_status.get_error_message());
                                        if ui.button("Close").clicked() {
                                            self.show_json_validation = false;
                                        }
                                    },
                                );
                            }
                            TimetableValidationStatus::Successful => {
                                ui.with_layout(
                                    Layout::top_down_justified(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(
                                                material_design_icons::MDI_CALENDAR_CHECK,
                                            )
                                                .size(32.0)
                                                .color(Color32::from_rgb(0x14, 0xae, 0x52)),
                                        );
                                        ui.label("Validation successful");
                                        ui.label("The timetable has been imported successfully. You may now proceed to the next step.");
                                        if ui.button("Close").clicked() {
                                            self.show_json_validation = false;
                                        }
                                    },
                                );
                            }
                        }
                    });
            }

            // Paths

            let path_list: Vec<&str> = vec![
                "A201", "S2-3", "S5-2", "X5A1", "X5B1", "S5-10", "S7-7", "B718",
            ];

            let mut segments: Vec<&[i32; 3]> = vec![];

            for i in &path_list {
                segments.push(&self.projection_coords[*i]);
            }

            // Paths window
            if self.show_path_window {
                Window::new("Path")
                    .open(&mut self.show_path_window)
                    .show(ctx, |ui| {
                        let mut path_string = String::new();
                        for i in &path_list {
                            path_string.push_str(i);
                            path_string.push_str(" → ");
                        }
                        path_string.pop();
                        path_string.pop();
                        path_string.pop();
                        ui.label(path_string);
                    });
            }

            // Import textures if uninitialized

            let mut textures: Vec<TextureHandle> = Vec::new();
            for i in 2..=8 {
                let texture_cur: &TextureHandle = self.textures[i].get_or_insert_with(|| {
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

            let current_floor_z = if self.selected_floor_index == 0 {
                0
            } else {
                ((self.selected_floor_index - 2) * 50) as i32
            };
            for (i, point) in segments.iter().enumerate() {
                if i != 0 {
                    ui.painter().circle_filled(
                        convert_pos(&rect, point, scale),
                        4.0,
                        if self.selected_floor_index == 0
                            || (current_floor_z >= point[2].min(segments[i - 1][2])
                            && current_floor_z <= point[2].max(segments[i - 1][2]))
                        {
                            self.active_path_color
                        } else {
                            self.inactive_path_color
                        },
                    );
                    ui.painter().line_segment(
                        [
                            convert_pos(&rect, segments[i - 1], scale),
                            convert_pos(&rect, point, scale),
                        ],
                        if self.selected_floor_index == 0
                            || (current_floor_z >= point[2].min(segments[i - 1][2])
                            && current_floor_z <= point[2].max(segments[i - 1][2]))
                        {
                            Stroke::new(4.0, self.active_path_color)
                        } else {
                            Stroke::new(4.0, self.inactive_path_color)
                        },
                    );
                } else {
                    ui.painter().circle_filled(
                        convert_pos(&rect, point, scale),
                        4.0,
                        if self.selected_floor_index == 0
                            || (current_floor_z == point[2].min(segments[i][2]))
                        {
                            self.active_path_color
                        } else {
                            self.inactive_path_color
                        },
                    );
                }
            }

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
        });
    }
}

fn load_image_from_path(path: &Path) -> Result<ColorImage, image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}

/// Converts 3D coordinates in projection-coords.yaml to 2D coordinates on screen.
fn convert_pos(rect: &Rect, pos: &[i32; 3], scale: f32) -> emath::Pos2 {
    /// Projection angle of the floor plan (radians)
    const ANGLE: f32 = PI / 6.0;

    (emath::vec2(
        rect.left() + 25.0 * ANGLE.cos() * scale,
        rect.top() + (50.0 + 350.0 + 25.0 * ANGLE.sin()) * scale,
    ) + emath::vec2(
        ((pos[0] as f32) * ANGLE.cos() + (pos[1] as f32) * ANGLE.cos()) * scale,
        ((pos[0] as f32) * ANGLE.sin() - (pos[1] as f32) * ANGLE.sin() - (pos[2] as f32)) * scale,
    ))
    .to_pos2()
}

fn convert_day_of_week(day: u32) -> String {
    match day {
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        _ => "Unknown",
    }
    .into()
}

fn convert_periods(index: usize) -> String {
    match index {
        1 => "Before P1",
        2 => "P1–P2",
        3 => "P2–P3",
        4 => "P3–P4",
        5 => "P4–P5",
        6 => "P5–P6",
        7 => "P6–Lunch",
        8 => "Lunch–P7",
        9 => "P7–P8",
        10 => "P8–P9",
        11 => "P9–P10",
        12 => "After P10",
        _ => "Unknown",
    }
    .into()
}

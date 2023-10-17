use std::{
    collections::HashMap,
    f32::consts::PI,
    fmt::Display,
    fs::{ self, File },
    io::{ BufRead, BufReader, Read, Write },
    path::{ Path, PathBuf },
    process::{ Command, Stdio },
    sync::{ Arc, Mutex },
    thread,
};

use egui::{
    color_picker,
    emath,
    pos2,
    CentralPanel,
    Color32,
    ColorImage,
    ComboBox,
    Grid,
    Layout,
    ProgressBar,
    Rect,
    RichText,
    Slider,
    Stroke,
    TextureHandle,
    Window,
};
use num_format::{ Locale, ToFormattedString };
use rfd::FileDialog;
use signal_child::Signalable;

use crate::{ md_icons::material_design_icons, setup_custom_fonts, setup_custom_styles };

#[derive(Default, Clone, PartialEq, Eq)]
enum PathDisplay {
    #[default]
    Shortest,
    Optimized,
}

impl Display for PathDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathDisplay::Shortest => write!(f, "Shortest route"),
            PathDisplay::Optimized => write!(f, "Optimized route"),
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

#[derive(Default, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct MainJSONOutputFile {
    iter: [u64; 5],
    indices: HashMap<u32, HashMap<usize, u128>>,
    routes: HashMap<String, HashMap<u32, HashMap<usize, String>>>,
}

#[derive(Default, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct SubJSONOutputFile {
    iter: u64,
    indices: HashMap<u32, HashMap<usize, u128>>,
    routes: HashMap<String, HashMap<u32, HashMap<usize, String>>>,
}

#[derive(Default, Clone, PartialEq, Eq)]
enum OptimizationStatus {
    #[default]
    ParamInput,
    Ready,
    Calculating,
    AbortSignal,
    Failed(String),
}

#[derive(Default)]
struct TimetableFileInfo {
    filename: String,
    filepath: PathBuf,
    student_count: Arc<Mutex<Option<i32>>>,
    session_count: Arc<Mutex<Option<i32>>>,
    validation_status: Arc<Mutex<TimetableValidationStatus>>,
    timetable: Arc<Mutex<Option<serde_json::Value>>>,
}

#[derive(Default, Clone, PartialEq, Eq)]
enum PathGenerationStatus {
    #[default]
    Ready,
    Generating(i32, String),
    Failed(String),
    LoadingJSON,
    Successful,
}

impl PathGenerationStatus {
    fn is_generating(&self) -> bool {
        matches!(self, PathGenerationStatus::Generating(_, _))
    }

    fn is_loading_json(&self) -> bool {
        matches!(self, PathGenerationStatus::LoadingJSON)
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
enum CongestionStatus {
    #[default]
    Ready,
    Generating(i32, String),
    Failed(String),
    GeneratingPI(i32, String),
    Successful,
}
impl CongestionStatus {
    fn is_generating(&self) -> bool {
        if let CongestionStatus::Generating(_, _) = self {
            true
        } else {
            matches!(self, CongestionStatus::GeneratingPI(_, _))
        }
    }
}

type Routes = HashMap<String, HashMap<u32, HashMap<usize, String>>>;
type CongestionPoint = HashMap<u32, HashMap<usize, HashMap<String, u32>>>;
type CongestionPath = HashMap<u32, HashMap<usize, HashMap<(String, String), u32>>>;

struct CongestionStatistics {
    point_count: HashMap<u32, HashMap<usize, Vec<u32>>>,
    path_count: HashMap<u32, HashMap<usize, Vec<u32>>>,
}

impl Default for CongestionStatistics {
    fn default() -> Self {
        Self {
            point_count: {
                let mut point_count = HashMap::new();
                for day in 1..=5 {
                    point_count.insert(day, HashMap::new());
                    for period in 0..=11 {
                        point_count
                            .get_mut(&day)
                            .unwrap()
                            .insert(period, vec![0; 7]);
                    }
                }
                point_count
            },
            path_count: {
                let mut path_count = HashMap::new();
                for day in 1..=5 {
                    path_count.insert(day, HashMap::new());
                    for period in 0..=11 {
                        path_count
                            .get_mut(&day)
                            .unwrap()
                            .insert(period, vec![0; 7]);
                    }
                }
                path_count
            },
        }
    }
}

pub struct OptiWayApp {
    selected_student: Option<String>,
    student_list: Arc<Mutex<Vec<String>>>,
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
    path_generation_status: Arc<Mutex<PathGenerationStatus>>,
    show_path_gen_window: bool,
    student_routes_shortest: Arc<Mutex<Option<Routes>>>,
    student_routes_optimized: Arc<Mutex<Option<Routes>>>,
    show_timetable_window: bool,
    show_congestion_window: bool,
    congestion_status: Arc<Mutex<CongestionStatus>>,
    congestion_point_data: Arc<Mutex<CongestionPoint>>,
    congestion_path_data: Arc<Mutex<CongestionPath>>,
    congestion_point_data_opt: Arc<Mutex<CongestionPoint>>,
    congestion_path_data_opt: Arc<Mutex<CongestionPath>>,
    maximum_congestion: Arc<Mutex<u32>>,
    maximum_congestion_opt: Arc<Mutex<u32>>,
    congestion_statistics: Arc<Mutex<CongestionStatistics>>,
    congestion_statistics_opt: Arc<Mutex<CongestionStatistics>>,
    show_congestion: bool,
    congestion_filter: u32,
    show_congestion_path: bool,
    show_congestion_point: bool,
    show_pi_window: bool,
    #[allow(unused)]
    shortest_paths_json: HashMap<String, String>,
    shortest_paths_content: Arc<Mutex<String>>,
    show_pi_shortest: bool,
    performance_indices_shortest: Arc<Mutex<HashMap<u32, HashMap<usize, u128>>>>,
    performance_indices_optimized: Arc<Mutex<HashMap<u32, HashMap<usize, u128>>>>,
    path_distances: Arc<Mutex<HashMap<String, HashMap<String, u32>>>>,
    optimization_status: Arc<Mutex<OptimizationStatus>>,
    show_optimization_window: bool,
    param_batch_size: u32,
    param_save_every: u32,
    param_use_shortest_path: bool,
    param_day: u32,
    param_filename: String,
    param_filepath: PathBuf,
    current_iter: Arc<Mutex<[u64; 5]>>,
    current_period_iter: Arc<Mutex<[u64; 5]>>,
    path_display: PathDisplay,
}

impl Default for OptiWayApp {
    fn default() -> Self {
        let mut floors = [false; 9];
        floors[0] = true;
        Self {
            selected_student: Default::default(),
            student_list: Default::default(),
            selected_period: 0,
            selected_day: 1,
            selected_floor: floors,
            selected_floor_index: 0,
            textures: vec![None; 9],
            inactive_brightness: 64,
            projection_coords: serde_yaml
                ::from_str(
                    std::fs
                        ::read_to_string("../assets/projection-coords-flatten.yaml")
                        .expect("Failed to read projection-coords-flatten.yaml")
                        .as_str()
                )
                .unwrap(),
            active_path_color: Color32::from_rgb(0xec, 0x6f, 0x27),
            inactive_path_color: Color32::from_gray(0x61),
            show_path_window: false,
            show_json_validation: false,
            timetable_file_info: Default::default(),
            student_number_search: Default::default(),
            path_generation_status: Default::default(),
            show_path_gen_window: false,
            student_routes_shortest: Default::default(),
            student_routes_optimized: Default::default(),
            show_timetable_window: false,
            show_congestion_window: false,
            congestion_status: Default::default(),
            congestion_point_data: Arc::new(
                Mutex::new({
                    let mut congestion_data = CongestionPoint::new();
                    for day in 1..=5 {
                        congestion_data.insert(day, HashMap::new());
                        for period in 0..=11 {
                            congestion_data.get_mut(&day).unwrap().insert(period, HashMap::new());
                        }
                    }
                    congestion_data
                })
            ),
            congestion_path_data: Arc::new(
                Mutex::new({
                    let mut congestion_data = CongestionPath::new();
                    for day in 1..=5 {
                        congestion_data.insert(day, HashMap::new());
                        for period in 0..=11 {
                            congestion_data.get_mut(&day).unwrap().insert(period, HashMap::new());
                        }
                    }
                    congestion_data
                })
            ),
            congestion_point_data_opt: Arc::new(
                Mutex::new({
                    let mut congestion_data = CongestionPoint::new();
                    for day in 1..=5 {
                        congestion_data.insert(day, HashMap::new());
                        for period in 0..=11 {
                            congestion_data.get_mut(&day).unwrap().insert(period, HashMap::new());
                        }
                    }
                    congestion_data
                })
            ),
            congestion_path_data_opt: Arc::new(
                Mutex::new({
                    let mut congestion_data = CongestionPath::new();
                    for day in 1..=5 {
                        congestion_data.insert(day, HashMap::new());
                        for period in 0..=11 {
                            congestion_data.get_mut(&day).unwrap().insert(period, HashMap::new());
                        }
                    }
                    congestion_data
                })
            ),
            congestion_statistics: Default::default(),
            congestion_statistics_opt: Default::default(),
            maximum_congestion: Default::default(),
            maximum_congestion_opt: Default::default(),
            show_congestion: false,
            congestion_filter: 0,
            show_congestion_path: true,
            show_congestion_point: true,
            show_pi_window: false,
            shortest_paths_json: serde_json
                ::from_reader::<_, HashMap<String, String>>(
                    File::open("./assets/shortest_paths.json").unwrap()
                )
                .unwrap(),
            performance_indices_shortest: {
                let mut performance_indices = HashMap::new();
                for day in 1..=5 {
                    performance_indices.insert(day, HashMap::new());
                    for period in 0..=11 {
                        performance_indices.get_mut(&day).unwrap().insert(period, 0);
                    }
                }
                Arc::new(Mutex::new(performance_indices))
            },
            performance_indices_optimized: {
                let mut performance_indices = HashMap::new();
                for day in 1..=5 {
                    performance_indices.insert(day, HashMap::new());
                    for period in 0..=11 {
                        performance_indices.get_mut(&day).unwrap().insert(period, 0);
                    }
                }
                Arc::new(Mutex::new(performance_indices))
            },
            show_pi_shortest: true,
            path_distances: Arc::new(
                Mutex::new({
                    let mut path_distances = HashMap::new();
                    // read paths.txt in the format [room1] [room2] [distance] [(ignore)]
                    let paths_file = File::open("./assets/paths.txt").unwrap();
                    let paths_file = std::io::BufReader::new(paths_file);
                    for line in paths_file.lines() {
                        let line = line.unwrap();
                        let mut line = line.split(' ');
                        let room1 = line.next().unwrap().to_owned();
                        let room2 = line.next().unwrap().to_owned();
                        let distance = line.next().unwrap().parse::<u32>().unwrap();
                        // insert if not present
                        if !path_distances.contains_key(&room1) {
                            path_distances.insert(room1.clone(), HashMap::new());
                        }
                        path_distances.get_mut(&room1).unwrap().insert(room2.clone(), distance);
                        if !path_distances.contains_key(&room2) {
                            path_distances.insert(room2.clone(), HashMap::new());
                        }
                        path_distances.get_mut(&room2).unwrap().insert(room1, distance);
                    }
                    path_distances
                })
            ),
            optimization_status: Default::default(),
            show_optimization_window: false,
            param_batch_size: 100,
            param_save_every: 2500,
            param_use_shortest_path: true,
            param_day: 1,
            param_filename: Default::default(),
            param_filepath: Default::default(),
            shortest_paths_content: Default::default(),
            current_iter: Default::default(),
            current_period_iter: Default::default(),
            path_display: Default::default(),
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

    fn show_path_generation_window(
        &mut self,
        ctx: &egui::Context,
        current_path_status: PathGenerationStatus
    ) {
        Window::new("Calculating Path").show(ctx, |ui| {
            ui.heading("Metadata");
            Grid::new("path_generation_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Algorithm");
                    ui.label("Minimize distance");
                    ui.end_row();

                    ui.label("Student count");
                    if
                        let Some(student_count) = *self.timetable_file_info.student_count
                            .lock()
                            .unwrap()
                    {
                        ui.label(format!("{}", student_count));
                    } else {
                        ui.label("—");
                    }
                    ui.end_row();

                    ui.label("Session count");
                    if
                        let Some(session_count) = *self.timetable_file_info.session_count
                            .lock()
                            .unwrap()
                    {
                        ui.label(format!("{}", session_count));
                    } else {
                        ui.label("—");
                    }
                    ui.end_row();
                });
            ui.separator();
            match current_path_status {
                PathGenerationStatus::Ready => {
                    *self.path_generation_status.lock().unwrap() = PathGenerationStatus::Generating(
                        0,
                        "Calculating path".to_owned()
                    );
                    let filepath = self.timetable_file_info.filepath.clone();
                    let path_generation_status_arc = self.path_generation_status.clone();
                    let student_paths_arc = self.student_routes_shortest.clone();
                    let shortest_paths_content_arc = self.shortest_paths_content.clone();
                    thread::spawn(move || {
                        *shortest_paths_content_arc.lock().unwrap() = run_floyd_algorithm_cpp(
                            filepath,
                            path_generation_status_arc,
                            student_paths_arc
                        );
                    });
                    // let shortest_paths_json = self.shortest_paths_json.clone();
                    // thread::spawn(move || run_floyd_algorithm_rust(filepath, shortest_paths_json, path_generation_status_arc, student_paths_arc));
                }
                PathGenerationStatus::Generating(_progress, message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(RichText::new(material_design_icons::MDI_MAP_SEARCH).size(32.0));
                        ui.label(message);
                        ui.spinner();
                    });
                }
                PathGenerationStatus::Failed(message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_SIGN_DIRECTION_REMOVE)
                                .size(32.0)
                                .color(Color32::from_rgb(0xe4, 0x37, 0x48))
                        );
                        ui.label("Path calculation failed");
                        ui.label(message);
                        if ui.button("Close").clicked() {
                            self.show_path_gen_window = false;
                        }
                    });
                }
                PathGenerationStatus::LoadingJSON => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_MAP_CHECK)
                                .size(32.0)
                                .color(Color32::from_rgb(0x14, 0xae, 0x52))
                        );
                        ui.label("Loading paths");
                        ui.label(
                            "The paths have been successfully calculated. OptiWay is now loading the paths."
                        );
                        ui.spinner();
                    });
                }
                PathGenerationStatus::Successful => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_MAP_CHECK)
                                .size(32.0)
                                .color(Color32::from_rgb(0x14, 0xae, 0x52))
                        );
                        ui.label("Path calculation successful");
                        ui.label(
                            "The paths have been successfully calculated and loaded. You may now view or export them."
                        );
                        if ui.button("Close").clicked() {
                            self.show_path_gen_window = false;
                        }
                    });
                }
            }
        });
    }

    fn show_optimization_window(
        &mut self,
        ctx: &egui::Context,
        current_optimization_status: OptimizationStatus
    ) {
        Window::new("Route Optimization").show(ctx, |ui| {
            match current_optimization_status {
                OptimizationStatus::ParamInput => {
                    ui.heading("Parameters");
                    ui.add(
                        Slider::new(&mut self.param_batch_size, 10..=200)
                            .step_by(10.0)
                            .text("Batch size")
                    );
                    ui.add(
                        Slider::new(&mut self.param_save_every, 100..=5000)
                            .step_by(100.0)
                            .text("Iterations per save")
                    );
                    // ComboBox::from_label("Day of Week")
                    //     .selected_text(convert_day_of_week(self.param_day))
                    //     .show_ui(ui, |ui| {
                    //         for i in 1..=5 {
                    //             ui.selectable_value(
                    //                 &mut self.param_day,
                    //                 i,
                    //                 convert_day_of_week(i),
                    //             );
                    //         }
                    //     });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.param_use_shortest_path, "Use shortest routes");
                        ui.add_enabled_ui(!self.param_use_shortest_path, |ui| {
                            if ui.button("Select route file").clicked() {
                                let file = FileDialog::new()
                                    .add_filter("JSON", &["json"])
                                    .set_directory("../")
                                    .pick_file();
                                if let Some(file) = file {
                                    self.param_filename = file
                                        .file_name()
                                        .unwrap()
                                        .to_str()
                                        .unwrap()
                                        .to_owned();
                                    self.param_filepath = file;
                                }
                            }
                        });
                        if !self.param_use_shortest_path {
                            ui.label(
                                if self.param_filename.is_empty() {
                                    "No file selected.".to_owned()
                                } else {
                                    self.param_filename.clone()
                                }
                            );
                        }
                    });
                    ui.separator();
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        if ui.button("Reset parameters").clicked() {
                            self.param_batch_size = 100;
                            self.param_save_every = 2500;
                            self.param_use_shortest_path = true;
                            self.param_day = 1;
                            self.param_filename = Default::default();
                            self.param_filepath = Default::default();
                        }
                        if ui.button("Start").clicked() {
                            self.show_pi_window = true;
                            self.show_pi_shortest = false;
                            *self.optimization_status.lock().unwrap() = OptimizationStatus::Ready;
                        }
                    });
                }
                OptimizationStatus::Ready => {
                    *self.optimization_status.lock().unwrap() = OptimizationStatus::Calculating;
                    let param_use_shortest_path = self.param_use_shortest_path;
                    let param_batch_size = self.param_batch_size;
                    let param_save_every = self.param_save_every;
                    for day in 1..=5 {
                        let param_day = day;
                        let mut param_filepath = self.param_filepath.clone();
                        let optimization_status_arc = self.optimization_status.clone();
                        let shortest_paths_content_arc = self.shortest_paths_content.clone();
                        let current_iter_arc = self.current_iter.clone();
                        let performance_indices_shortest_json = serde_json
                            ::to_string(&*self.performance_indices_shortest.lock().unwrap())
                            .unwrap();
                        let performance_indices_optimized_arc =
                            self.performance_indices_optimized.clone();
                        let performance_indices_shortest_arc =
                            self.performance_indices_shortest.clone();
                        let current_period_iter_arc = self.current_period_iter.clone();
                        if param_use_shortest_path {
                            self.param_filepath = fs::canonicalize("./bin/routes.json").unwrap();
                            self.param_filename = "routes.json".to_owned();
                            self.param_use_shortest_path = false;
                        } else {
                            let mut file = File::open(&param_filepath).unwrap();
                            let mut content = String::new();
                            file.read_to_string(&mut content).unwrap();
                            let json: MainJSONOutputFile = serde_json::from_str(&content).unwrap();
                            for day in 1..=5 {
                                *self.current_iter
                                    .lock()
                                    .unwrap()
                                    .get_mut((day - 1) as usize)
                                    .unwrap() = *json.iter.get((day - 1) as usize).unwrap();
                            }
                            *self.performance_indices_optimized.lock().unwrap() = json.indices;
                        }
                        thread::spawn(move || {
                            if param_use_shortest_path {
                                let content =
                                    "{\"iter\":[0,0,0,0,0],\"indices\":".to_owned() +
                                    &performance_indices_shortest_json +
                                    ",\"routes\":" +
                                    &shortest_paths_content_arc.lock().unwrap().to_owned() +
                                    "}";
                                *performance_indices_optimized_arc.lock().unwrap() =
                                    performance_indices_shortest_arc.lock().unwrap().clone();
                                let mut file = File::create("./bin/routes.json").unwrap();
                                file.write_all(content.as_bytes()).unwrap();
                                param_filepath = fs::canonicalize("./bin/routes.json").unwrap();
                            }
                            let bin_dir = fs::canonicalize("./bin").unwrap();
                            if
                                let Ok(mut opt_command) = Command::new("./optimization.out")
                                    .current_dir(bin_dir)
                                    .stdout(Stdio::piped())
                                    .stdin(Stdio::piped())
                                    .args([
                                        "-f",
                                        param_filepath.to_str().unwrap(),
                                        "-b",
                                        &param_batch_size.to_string(),
                                        "-s",
                                        &(param_save_every / 5).to_string(),
                                        "-d",
                                        &param_day.to_string(),
                                    ])
                                    .spawn()
                            {
                                if let Some(stdout) = opt_command.stdout.take() {
                                    let reader = BufReader::new(stdout);
                                    reader
                                        .lines()
                                        .map_while(Result::ok)
                                        .for_each(|line| {
                                            let report: Vec<&str> = line.split(' ').collect();
                                            if report[0] != "1" {
                                                (*current_iter_arc.lock().unwrap())[
                                                    (param_day - 1) as usize
                                                ] = report[1].parse::<u64>().unwrap();
                                                if
                                                    optimization_status_arc
                                                        .lock()
                                                        .unwrap()
                                                        .clone() == OptimizationStatus::AbortSignal
                                                {
                                                    opt_command
                                                        .interrupt()
                                                        .expect(
                                                            "Failed to interrupt optimization algorithm"
                                                        );
                                                }
                                                (*current_period_iter_arc.lock().unwrap())[
                                                    (param_day - 1) as usize
                                                ] = report[3].parse::<u64>().unwrap();
                                                *performance_indices_optimized_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get_mut(&param_day)
                                                    .unwrap()
                                                    .get_mut(
                                                        &(
                                                            report[3]
                                                                .parse::<u64>()
                                                                .unwrap() as usize
                                                        )
                                                    )
                                                    .unwrap() = report[4].parse::<u128>().unwrap();
                                            }
                                        });
                                } else {
                                    *optimization_status_arc.lock().unwrap() =
                                        OptimizationStatus::Failed(
                                            "Optimization algorithm terminated unexpectedly".to_owned()
                                        );
                                }
                            } else {
                                *optimization_status_arc.lock().unwrap() =
                                    OptimizationStatus::Failed(
                                        "Failed to start optimization algorithm".to_owned()
                                    );
                            }
                        });
                    }
                }
                OptimizationStatus::Calculating => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(RichText::new(material_design_icons::MDI_COG).size(32.0));
                        ui.label("Optimizing paths");
                        ui.label(
                            format!(
                                "Total Iteration: {}",
                                self.current_iter
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .sum::<u64>()
                                    .to_formatted_string(&Locale::fr)
                            )
                        );
                        ui.spinner();
                        if ui.button("Pause").clicked() {
                            *self.optimization_status.lock().unwrap() =
                                OptimizationStatus::AbortSignal;
                            // read from routes.json and load the json
                            let mut file = File::open("./bin/routes.json").unwrap();
                            let mut content = String::new();
                            file.read_to_string(&mut content).unwrap();
                            let mut json: MainJSONOutputFile = serde_json
                                ::from_str(&content)
                                .unwrap();
                            let mut sub_jsons: [SubJSONOutputFile; 5] = Default::default();
                            for i in 1..=5 {
                                let mut file = File::open(
                                    format!("./bin/routes.json_{}.json", i)
                                ).unwrap();
                                let mut content = String::new();
                                file.read_to_string(&mut content).unwrap();
                                sub_jsons[i - 1] = serde_json::from_str(&content).unwrap();
                                json.iter[i - 1] = sub_jsons[i - 1].iter;
                                *json.indices.get_mut(&(i as u32)).unwrap() = sub_jsons[
                                    i - 1
                                ].indices
                                    .get(&(i as u32))
                                    .unwrap()
                                    .clone();
                                for student_number in sub_jsons[i - 1].routes.keys() {
                                    *json.routes
                                        .get_mut(student_number)
                                        .unwrap()
                                        .get_mut(&(i as u32))
                                        .unwrap() = sub_jsons[i - 1].routes
                                        .get(student_number)
                                        .unwrap()
                                        .get(&(i as u32))
                                        .unwrap()
                                        .clone();
                                }
                            }
                            let mut file = File::create("./bin/routes.json").unwrap();
                            file.write_all(
                                serde_json::to_string(&json).unwrap().as_bytes()
                            ).unwrap();
                            *self.student_routes_optimized.lock().unwrap() = Some(json.routes);
                        }
                    });
                }
                OptimizationStatus::AbortSignal => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_COG_PAUSE)
                                .size(32.0)
                                .color(Color32::from_rgb(0xff, 0xc1, 0x07))
                        );
                        ui.label("Optimization paused");
                        ui.label("You may resume optimization at any time.");
                        if ui.button("Resume").clicked() {
                            *self.optimization_status.lock().unwrap() = OptimizationStatus::Ready;
                        }
                        if ui.button("Close").clicked() {
                            self.show_optimization_window = false;
                            *self.optimization_status.lock().unwrap() =
                                OptimizationStatus::ParamInput;
                        }
                    });
                }
                OptimizationStatus::Failed(message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_COG_OFF)
                                .size(32.0)
                                .color(Color32::from_rgb(0xe4, 0x37, 0x48))
                        );
                        ui.label("Route optimization failed");
                        ui.label(message);
                        if ui.button("Close").clicked() {
                            self.show_optimization_window = false;
                        }
                    });
                }
            }
        });
    }

    fn show_congestion_window(
        &mut self,
        ctx: &egui::Context,
        current_congestion_status: CongestionStatus
    ) {
        Window::new("Congestion Evaluation").show(ctx, |ui| {
            match current_congestion_status {
                CongestionStatus::Ready => {
                    *self.maximum_congestion.lock().unwrap() = 0;
                    *self.congestion_status.lock().unwrap() = CongestionStatus::Generating(
                        0,
                        "Evaluating congestion".to_owned()
                    );
                    *self.congestion_statistics.lock().unwrap() = Default::default();
                    *self.congestion_point_data.lock().unwrap() = {
                        let mut congestion_data = CongestionPoint::new();
                        for day in 1..=5 {
                            congestion_data.insert(day, HashMap::new());
                            for period in 0..=11 {
                                congestion_data
                                    .get_mut(&day)
                                    .unwrap()
                                    .insert(period, HashMap::new());
                            }
                        }
                        congestion_data
                    };
                    *self.congestion_point_data_opt.lock().unwrap() = {
                        let mut congestion_data = CongestionPoint::new();
                        for day in 1..=5 {
                            congestion_data.insert(day, HashMap::new());
                            for period in 0..=11 {
                                congestion_data
                                    .get_mut(&day)
                                    .unwrap()
                                    .insert(period, HashMap::new());
                            }
                        }
                        congestion_data
                    };
                    *self.congestion_path_data.lock().unwrap() = {
                        let mut congestion_data = CongestionPath::new();
                        for day in 1..=5 {
                            congestion_data.insert(day, HashMap::new());
                            for period in 0..=11 {
                                congestion_data
                                    .get_mut(&day)
                                    .unwrap()
                                    .insert(period, HashMap::new());
                            }
                        }
                        congestion_data
                    };
                    *self.congestion_path_data_opt.lock().unwrap() = {
                        let mut congestion_data = CongestionPath::new();
                        for day in 1..=5 {
                            congestion_data.insert(day, HashMap::new());
                            for period in 0..=11 {
                                congestion_data
                                    .get_mut(&day)
                                    .unwrap()
                                    .insert(period, HashMap::new());
                            }
                        }
                        congestion_data
                    };
                    let congestion_point_data_arc = self.congestion_point_data.clone();
                    let congestion_path_data_arc = self.congestion_path_data.clone();
                    let congestion_point_data_opt_arc = self.congestion_point_data_opt.clone();
                    let congestion_path_data_opt_arc = self.congestion_path_data_opt.clone();
                    let congestion_status_arc = self.congestion_status.clone();
                    let max_congestion_arc = self.maximum_congestion.clone();
                    let max_congestion_opt_arc = self.maximum_congestion_opt.clone();
                    let congestion_statistics_arc = self.congestion_statistics.clone();
                    let congestion_statistics_opt_arc = self.congestion_statistics_opt.clone();
                    let path_distances_arc = self.path_distances.clone();
                    let mut rooms: Vec<String> = self.projection_coords
                        .clone()
                        .keys()
                        .map(|k| k.to_owned())
                        .collect();
                    let performance_indices_shortest_arc =
                        self.performance_indices_shortest.clone();
                    rooms.push("G".to_owned());
                    let student_routes = self.student_routes_shortest.lock().unwrap().clone();
                    if student_routes.is_none() {
                        *congestion_status_arc.lock().unwrap() = CongestionStatus::Failed(
                            "No path data available".to_owned()
                        );
                        return;
                    }
                    let student_routes = student_routes.unwrap();
                    let student_routes_opt = self.student_routes_optimized.lock().unwrap().clone();
                    thread::spawn(move || {
                        if let Some(student_routes) = student_routes_opt {
                            for day in 1..=5 {
                                for period in 0..=11 {
                                    for room in &rooms {
                                        congestion_point_data_opt_arc
                                            .lock()
                                            .unwrap()
                                            .get_mut(&day)
                                            .unwrap()
                                            .get_mut(&period)
                                            .unwrap()
                                            .insert(room.to_owned(), 0);
                                    }
                                }
                            }
                            for student in &student_routes {
                                for day in 1..=5 {
                                    for period in 0..=11 {
                                        let rooms = student_routes
                                            .get(student.0)
                                            .unwrap()
                                            .get(&day)
                                            .unwrap()
                                            .get(&period)
                                            .unwrap()
                                            .split(' ')
                                            .collect::<Vec<&str>>();
                                        let mut previous_room = "";
                                        for room in rooms {
                                            if room.is_empty() || room == "G" {
                                                continue;
                                            }
                                            if !previous_room.is_empty() {
                                                let contains = congestion_path_data_opt_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get(&day)
                                                    .unwrap()
                                                    .get(&period)
                                                    .unwrap()
                                                    .contains_key(
                                                        &(previous_room.to_owned(), room.to_owned())
                                                    );
                                                if contains {
                                                    *congestion_path_data_opt_arc
                                                        .lock()
                                                        .unwrap()
                                                        .get_mut(&day)
                                                        .unwrap()
                                                        .get_mut(&period)
                                                        .unwrap()
                                                        .get_mut(
                                                            &(
                                                                previous_room.to_owned(),
                                                                room.to_owned(),
                                                            )
                                                        )
                                                        .unwrap() += 1;
                                                } else {
                                                    congestion_path_data_opt_arc
                                                        .lock()
                                                        .unwrap()
                                                        .get_mut(&day)
                                                        .unwrap()
                                                        .get_mut(&period)
                                                        .unwrap()
                                                        .insert(
                                                            (
                                                                previous_room.to_owned(),
                                                                room.to_owned(),
                                                            ),
                                                            1
                                                        );
                                                }
                                                let contains = congestion_path_data_opt_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get(&day)
                                                    .unwrap()
                                                    .get(&period)
                                                    .unwrap()
                                                    .contains_key(
                                                        &(room.to_owned(), previous_room.to_owned())
                                                    );
                                                if contains {
                                                    *congestion_path_data_opt_arc
                                                        .lock()
                                                        .unwrap()
                                                        .get_mut(&day)
                                                        .unwrap()
                                                        .get_mut(&period)
                                                        .unwrap()
                                                        .get_mut(
                                                            &(
                                                                room.to_owned(),
                                                                previous_room.to_owned(),
                                                            )
                                                        )
                                                        .unwrap() += 1;
                                                } else {
                                                    congestion_path_data_opt_arc
                                                        .lock()
                                                        .unwrap()
                                                        .get_mut(&day)
                                                        .unwrap()
                                                        .get_mut(&period)
                                                        .unwrap()
                                                        .insert(
                                                            (
                                                                room.to_owned(),
                                                                previous_room.to_owned(),
                                                            ),
                                                            1
                                                        );
                                                }
                                            }
                                            previous_room = room;
                                            *congestion_point_data_opt_arc
                                                .lock()
                                                .unwrap()
                                                .get_mut(&day)
                                                .unwrap()
                                                .get_mut(&period)
                                                .unwrap()
                                                .get_mut(room)
                                                .unwrap() += 1;
                                            let cur_max_congestion = *max_congestion_opt_arc
                                                .lock()
                                                .unwrap();
                                            *max_congestion_opt_arc.lock().unwrap() =
                                                cur_max_congestion.max(
                                                    *congestion_point_data_opt_arc
                                                        .lock()
                                                        .unwrap()
                                                        .get_mut(&day)
                                                        .unwrap()
                                                        .get_mut(&period)
                                                        .unwrap()
                                                        .get_mut(room)
                                                        .unwrap()
                                                );
                                        }
                                    }
                                }
                            }
                            for day in 1..=5 {
                                for period in 0..=11 {
                                    for room in &rooms {
                                        let room_congestion = *congestion_point_data_opt_arc
                                            .lock()
                                            .unwrap()
                                            .get_mut(&day)
                                            .unwrap()
                                            .get_mut(&period)
                                            .unwrap()
                                            .get_mut(room)
                                            .unwrap();
                                        congestion_statistics_opt_arc
                                            .lock()
                                            .unwrap()
                                            .point_count.get_mut(&day)
                                            .unwrap()
                                            .get_mut(&period)
                                            .unwrap()[congestion_range_index(room_congestion)] += 1;
                                    }
                                }
                            }
                            for day in 1..=5 {
                                for period in 0..=11 {
                                    congestion_path_data_opt_arc
                                        .lock()
                                        .unwrap()
                                        .get_mut(&day)
                                        .unwrap()
                                        .get_mut(&period)
                                        .unwrap()
                                        .iter_mut()
                                        .for_each(|(_, congestion)| {
                                            congestion_statistics_opt_arc
                                                .lock()
                                                .unwrap()
                                                .path_count.get_mut(&day)
                                                .unwrap()
                                                .get_mut(&period)
                                                .unwrap()[congestion_range_index(*congestion)] += 1;
                                        });
                                }
                            }
                        }
                        for day in 1..=5 {
                            for period in 0..=11 {
                                for room in &rooms {
                                    congestion_point_data_arc
                                        .lock()
                                        .unwrap()
                                        .get_mut(&day)
                                        .unwrap()
                                        .get_mut(&period)
                                        .unwrap()
                                        .insert(room.to_owned(), 0);
                                }
                            }
                        }
                        for student in &student_routes {
                            for day in 1..=5 {
                                for period in 0..=11 {
                                    let rooms = student_routes
                                        .get(student.0)
                                        .unwrap()
                                        .get(&day)
                                        .unwrap()
                                        .get(&period)
                                        .unwrap()
                                        .split(' ')
                                        .collect::<Vec<&str>>();
                                    let mut previous_room = "";
                                    for room in rooms {
                                        if room.is_empty() || room == "G" {
                                            continue;
                                        }
                                        if !previous_room.is_empty() {
                                            let contains = congestion_path_data_arc
                                                .lock()
                                                .unwrap()
                                                .get(&day)
                                                .unwrap()
                                                .get(&period)
                                                .unwrap()
                                                .contains_key(
                                                    &(previous_room.to_owned(), room.to_owned())
                                                );
                                            if contains {
                                                *congestion_path_data_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get_mut(&day)
                                                    .unwrap()
                                                    .get_mut(&period)
                                                    .unwrap()
                                                    .get_mut(
                                                        &(previous_room.to_owned(), room.to_owned())
                                                    )
                                                    .unwrap() += 1;
                                            } else {
                                                congestion_path_data_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get_mut(&day)
                                                    .unwrap()
                                                    .get_mut(&period)
                                                    .unwrap()
                                                    .insert(
                                                        (previous_room.to_owned(), room.to_owned()),
                                                        1
                                                    );
                                            }
                                            let contains = congestion_path_data_arc
                                                .lock()
                                                .unwrap()
                                                .get(&day)
                                                .unwrap()
                                                .get(&period)
                                                .unwrap()
                                                .contains_key(
                                                    &(room.to_owned(), previous_room.to_owned())
                                                );
                                            if contains {
                                                *congestion_path_data_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get_mut(&day)
                                                    .unwrap()
                                                    .get_mut(&period)
                                                    .unwrap()
                                                    .get_mut(
                                                        &(room.to_owned(), previous_room.to_owned())
                                                    )
                                                    .unwrap() += 1;
                                            } else {
                                                congestion_path_data_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get_mut(&day)
                                                    .unwrap()
                                                    .get_mut(&period)
                                                    .unwrap()
                                                    .insert(
                                                        (room.to_owned(), previous_room.to_owned()),
                                                        1
                                                    );
                                            }
                                        }
                                        previous_room = room;
                                        *congestion_point_data_arc
                                            .lock()
                                            .unwrap()
                                            .get_mut(&day)
                                            .unwrap()
                                            .get_mut(&period)
                                            .unwrap()
                                            .get_mut(room)
                                            .unwrap() += 1;
                                        let cur_max_congestion = *max_congestion_arc
                                            .lock()
                                            .unwrap();
                                        *max_congestion_arc.lock().unwrap() =
                                            cur_max_congestion.max(
                                                *congestion_point_data_arc
                                                    .lock()
                                                    .unwrap()
                                                    .get_mut(&day)
                                                    .unwrap()
                                                    .get_mut(&period)
                                                    .unwrap()
                                                    .get_mut(room)
                                                    .unwrap()
                                            );
                                    }
                                }
                            }
                        }
                        for day in 1..=5 {
                            for period in 0..=11 {
                                for room in &rooms {
                                    let room_congestion = *congestion_point_data_arc
                                        .lock()
                                        .unwrap()
                                        .get_mut(&day)
                                        .unwrap()
                                        .get_mut(&period)
                                        .unwrap()
                                        .get_mut(room)
                                        .unwrap();
                                    congestion_statistics_arc
                                        .lock()
                                        .unwrap()
                                        .point_count.get_mut(&day)
                                        .unwrap()
                                        .get_mut(&period)
                                        .unwrap()[congestion_range_index(room_congestion)] += 1;
                                }
                            }
                        }
                        for day in 1..=5 {
                            for period in 0..=11 {
                                congestion_path_data_arc
                                    .lock()
                                    .unwrap()
                                    .get_mut(&day)
                                    .unwrap()
                                    .get_mut(&period)
                                    .unwrap()
                                    .iter_mut()
                                    .for_each(|(_, congestion)| {
                                        congestion_statistics_arc
                                            .lock()
                                            .unwrap()
                                            .path_count.get_mut(&day)
                                            .unwrap()
                                            .get_mut(&period)
                                            .unwrap()[congestion_range_index(*congestion)] += 1;
                                    });
                            }
                        }
                        *congestion_status_arc.lock().unwrap() = CongestionStatus::GeneratingPI(
                            0,
                            "Calculating performance indices".to_owned()
                        );
                        let mut performance_indices_shortest = HashMap::new();
                        let path_distances = path_distances_arc.lock().unwrap().clone();
                        for day in 1..=5 {
                            performance_indices_shortest.insert(day, HashMap::new());
                            for period in 0..=11 {
                                let mut index = 0f64;
                                for student in &student_routes {
                                    let rooms = student_routes
                                        .get(student.0)
                                        .unwrap()
                                        .get(&day)
                                        .unwrap()
                                        .get(&period)
                                        .unwrap()
                                        .split(' ')
                                        .collect::<Vec<&str>>();
                                    let mut previous_room = "";
                                    for room in rooms {
                                        if room.is_empty() || room == "G" {
                                            continue;
                                        }
                                        if !previous_room.is_empty() {
                                            let path_distance = *path_distances
                                                .get(previous_room)
                                                .unwrap()
                                                .get(room)
                                                .unwrap_or_else(|| {
                                                    println!(
                                                        "{:?}",
                                                        path_distances.get(previous_room).unwrap()
                                                    );
                                                    panic!(
                                                        "Path distance not found: {} -> {}",
                                                        previous_room,
                                                        room
                                                    )
                                                }) as f64;
                                            let path_congestion = *congestion_path_data_arc
                                                .lock()
                                                .unwrap()
                                                .get(&day)
                                                .unwrap()
                                                .get(&period)
                                                .unwrap()
                                                .get(&(previous_room.to_owned(), room.to_owned()))
                                                .unwrap() as f64;
                                            index +=
                                                path_distance *
                                                (2.0 + ((path_congestion - 300.0) / 200.0).tanh());
                                        }
                                        previous_room = room;
                                    }
                                }
                                performance_indices_shortest
                                    .get_mut(&day)
                                    .unwrap()
                                    .insert(period, index as u128);
                            }
                        }
                        // println!("{}", serde_json::to_string(&performance_indices_shortest).unwrap());
                        *performance_indices_shortest_arc.lock().unwrap() =
                            performance_indices_shortest;
                        *congestion_status_arc.lock().unwrap() = CongestionStatus::Successful;
                    });
                }
                CongestionStatus::GeneratingPI(_, message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(RichText::new(material_design_icons::MDI_TIMER).size(32.0));
                        ui.label(message);
                        ui.spinner();
                    });
                }
                CongestionStatus::Generating(_, message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_TRAFFIC_LIGHT).size(32.0)
                        );
                        ui.label(message);
                        ui.spinner();
                    });
                }
                CongestionStatus::Failed(message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_TRAFFIC_LIGHT)
                                .size(32.0)
                                .color(Color32::from_rgb(0xe4, 0x37, 0x48))
                        );
                        ui.label("Congestion evaluation failed");
                        ui.label(message);
                        if ui.button("Close").clicked() {
                            self.show_congestion_window = false;
                        }
                    });
                }
                CongestionStatus::Successful => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_TRAFFIC_LIGHT)
                                .size(32.0)
                                .color(Color32::from_rgb(0x14, 0xae, 0x52))
                        );
                        ui.label("Congestion evaluation successful");
                        ui.label(
                            "The congestion has been successfully evaluated and loaded. You may now view them on the projection."
                        );
                        ui.label(
                            format!(
                                "Maximum congestion: {}",
                                self.maximum_congestion.lock().unwrap()
                            )
                        );
                        if ui.button("Close").clicked() {
                            self.show_congestion_window = false;
                        }
                    });
                }
            }
        });
    }

    fn show_json_validation_window(
        &mut self,
        ctx: &egui::Context,
        current_validation_status: TimetableValidationStatus
    ) {
        Window::new("Timetable Validation").show(ctx, |ui| {
            ui.heading("Metadata");
            Grid::new("timetable_validation_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Filename");
                    ui.label(self.timetable_file_info.filename.to_string());
                    ui.end_row();

                    ui.label("Student count");
                    if
                        let Some(student_count) = *self.timetable_file_info.student_count
                            .lock()
                            .unwrap()
                    {
                        ui.label(format!("{}", student_count));
                    } else {
                        ui.label("—");
                    }
                    ui.end_row();

                    ui.label("Session count");
                    if
                        let Some(session_count) = *self.timetable_file_info.session_count
                            .lock()
                            .unwrap()
                    {
                        ui.label(format!("{}", session_count));
                    } else {
                        ui.label("—");
                    }
                    ui.end_row();
                });
            ui.separator();
            match current_validation_status {
                TimetableValidationStatus::Ready => {
                    *self.timetable_file_info.validation_status.clone().lock().unwrap() =
                        TimetableValidationStatus::Validating(0, "Ready to validate".to_owned());
                    let filepath = self.timetable_file_info.filepath.clone();
                    let projection_coords = self.projection_coords.clone();
                    let validation_status_arc = self.timetable_file_info.validation_status.clone();
                    let student_count_arc = self.timetable_file_info.student_count.clone();
                    let session_count_arc = self.timetable_file_info.session_count.clone();
                    let timetable_arc = self.timetable_file_info.timetable.clone();
                    let student_list_arc = self.student_list.clone();
                    thread::spawn(move || {
                        let mut rooms: Vec<String> = projection_coords
                            .keys()
                            .map(|k| k.to_owned())
                            .collect();
                        *validation_status_arc.lock().unwrap() =
                            TimetableValidationStatus::Validating(
                                0,
                                "Validating student numbers...".to_owned()
                            );
                        rooms.push("G".into());
                        let mut timetable_file = std::fs::File::open(filepath).unwrap();
                        let mut timetable_json = String::new();
                        if timetable_file.read_to_string(&mut timetable_json).is_err() {
                            *validation_status_arc.lock().unwrap() =
                                TimetableValidationStatus::Failed(
                                    "Failed to read timetable file".to_owned()
                                );
                            return;
                        }
                        let timetable: serde_json::Value = match
                            serde_json::from_str(&timetable_json)
                        {
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
                                    "Invalid timetable file format: the JSON file is not a map".to_owned()
                                );
                            return;
                        }
                        for student_key in timetable.as_object().unwrap().keys() {
                            if
                                student_key.chars().all(char::is_numeric) &&
                                4 <= student_key.len() &&
                                student_key.len() <= 5
                            {
                                student_count += 1;
                                *student_count_arc.lock().unwrap() = Some(student_count);
                            } else {
                                *validation_status_arc.lock().unwrap() =
                                    TimetableValidationStatus::Failed(
                                        format!("Invalid student number: \"{}\"", student_key)
                                    );
                                return;
                            }
                        }

                        *validation_status_arc.lock().unwrap() =
                            TimetableValidationStatus::Validating(
                                10,
                                "Validating days of the week...".to_owned()
                            );
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
                                if
                                    day_key.chars().all(char::is_numeric) &&
                                    day_key.parse::<i32>().unwrap() <= 5 &&
                                    day_key.parse::<i32>().unwrap() >= 1
                                {
                                    days_of_week[
                                        (day_key.parse::<i32>().unwrap() as usize) - 1
                                    ] = true;
                                } else {
                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Failed(
                                            format!(
                                                "Student {} has an invalid day of week: \"{}\"",
                                                student_key,
                                                day_key
                                            )
                                        );
                                    return;
                                }
                            }
                            if days_of_week.contains(&false) {
                                *validation_status_arc.lock().unwrap() =
                                    TimetableValidationStatus::Failed(
                                        format!(
                                            "Student {} has an incomplete timetable: missing day {}",
                                            student_key,
                                            days_of_week
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, &b)| !b)
                                                .map(|(i, _)| i + 1)
                                                .collect::<Vec<usize>>()
                                                .iter()
                                                .map(|i| i.to_string())
                                                .collect::<Vec<String>>()
                                                .join(", ")
                                        )
                                    );
                                return;
                            }
                        }

                        *validation_status_arc.lock().unwrap() =
                            TimetableValidationStatus::Validating(
                                20,
                                "Validating periods...".to_owned()
                            );
                        for (student_key, week_timetable) in timetable.as_object().unwrap() {
                            for (day_key, day_timetable) in week_timetable.as_object().unwrap() {
                                if day_timetable.as_object().is_none() {
                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Failed(
                                            format!(
                                                "Invalid timetable file format: student {}'s timetable on day {} is not a map",
                                                student_key,
                                                day_key
                                            )
                                        );
                                    return;
                                }
                                let mut periods = [false; 10];
                                for period_key in day_timetable.as_object().unwrap().keys() {
                                    if
                                        period_key.chars().all(char::is_numeric) &&
                                        period_key.parse::<i32>().unwrap() <= 10 &&
                                        period_key.parse::<i32>().unwrap() >= 1
                                    {
                                        periods[
                                            (period_key.parse::<i32>().unwrap() as usize) - 1
                                        ] = true;
                                    } else {
                                        *validation_status_arc.lock().unwrap() =
                                            TimetableValidationStatus::Failed(
                                                format!(
                                                    "Student {} has an invalid period on day {}: \"{}\"",
                                                    student_key,
                                                    day_key,
                                                    period_key
                                                )
                                            );
                                        return;
                                    }
                                }
                                if periods.contains(&false) {
                                    *validation_status_arc.lock().unwrap() =
                                        TimetableValidationStatus::Failed(
                                            format!(
                                                "Student {} has an incomplete timetable on day {}: missing periods {}",
                                                student_key,
                                                day_key,
                                                periods
                                                    .iter()
                                                    .enumerate()
                                                    .filter(|(_, &b)| !b)
                                                    .map(|(i, _)| i + 1)
                                                    .collect::<Vec<usize>>()
                                                    .iter()
                                                    .map(|i| i.to_string())
                                                    .collect::<Vec<String>>()
                                                    .join(", ")
                                            )
                                        );
                                    return;
                                }
                            }
                        }

                        *validation_status_arc.lock().unwrap() =
                            TimetableValidationStatus::Validating(
                                20,
                                "Validating classrooms...".to_owned()
                            );
                        let mut sessions = 0;
                        for (student_key, week_timetable) in timetable.as_object().unwrap() {
                            for (day_key, day_timetable) in week_timetable.as_object().unwrap() {
                                for (period_key, room) in day_timetable.as_object().unwrap() {
                                    if !rooms.contains(&room.as_str().unwrap().to_owned()) {
                                        *validation_status_arc.lock().unwrap() =
                                            TimetableValidationStatus::Failed(
                                                format!(
                                                    "Student {} has an invalid classroom on day {} period {}: {}",
                                                    student_key,
                                                    day_key,
                                                    period_key,
                                                    room
                                                )
                                            );
                                        return;
                                    }
                                    sessions += 1;
                                    if sessions % 1000 == 0 {
                                        *session_count_arc.lock().unwrap() = Some(sessions);
                                        *validation_status_arc.lock().unwrap() =
                                            TimetableValidationStatus::Validating(
                                                20 +
                                                    (
                                                        (((sessions as f32) /
                                                            ((student_count * 10 * 5) as f32)) *
                                                            80.0) as i32
                                                    ),
                                                "Validating classrooms...".to_owned()
                                            );
                                    }
                                }
                            }
                        }
                        *timetable_arc.lock().unwrap() = Some(timetable.clone());

                        *student_list_arc.lock().unwrap() = timetable
                            .as_object()
                            .unwrap()
                            .keys()
                            .map(|k| k.to_owned())
                            .collect();

                        *validation_status_arc.lock().unwrap() =
                            TimetableValidationStatus::Successful;
                    });
                }
                TimetableValidationStatus::Validating(progress, message) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_CALENDAR_SEARCH).size(32.0)
                        );
                        ui.label(message);
                        ui.add(ProgressBar::new((progress as f32) / 100.0));
                    });
                }
                TimetableValidationStatus::Failed(_) => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_CALENDAR_REMOVE)
                                .size(32.0)
                                .color(Color32::from_rgb(0xe4, 0x37, 0x48))
                        );
                        ui.label("Validation failed");
                        ui.label(current_validation_status.get_error_message());
                        if ui.button("Close").clicked() {
                            self.show_json_validation = false;
                        }
                    });
                }
                TimetableValidationStatus::Successful => {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(material_design_icons::MDI_CALENDAR_CHECK)
                                .size(32.0)
                                .color(Color32::from_rgb(0x14, 0xae, 0x52))
                        );
                        ui.label("Validation successful");
                        ui.label(
                            "The timetable has been imported successfully. You may now proceed to the next step."
                        );
                        if ui.button("Close").clicked() {
                            self.show_json_validation = false;
                        }
                    });
                }
            }
        });
    }

    fn show_timetable_window(&mut self, ctx: &egui::Context) {
        Window::new("Timetable")
            .open(&mut self.show_timetable_window)
            .show(ctx, |ui| {
                if
                    self.timetable_file_info.validation_status.lock().unwrap().clone() ==
                    TimetableValidationStatus::Successful
                {
                    if let Some(student) = &self.selected_student {
                        ui.label(format!("{}'s Timetable", student));
                        egui::Grid
                            ::new("timetable_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Period");
                                ui.label("Monday");
                                ui.label("Tuesday");
                                ui.label("Wednesday");
                                ui.label("Thursday");
                                ui.label("Friday");
                                ui.end_row();

                                let timetable = self.timetable_file_info.timetable.lock().unwrap();

                                for period in 1..=10 {
                                    ui.label(format!("Period {}", period));
                                    for weekday in 1..=5 {
                                        let mut session = timetable
                                            .as_ref()
                                            .unwrap()
                                            .get(student)
                                            .unwrap()
                                            .get(weekday.to_string())
                                            .unwrap()
                                            .get(period.to_string())
                                            .unwrap()
                                            .as_str()
                                            .unwrap();
                                        if session == "G" {
                                            session = " ";
                                        }
                                        ui.label(session);
                                    }
                                    ui.end_row();
                                }
                            });
                    } else {
                        ui.label("No student selected.");
                    }
                } else {
                    ui.label("Timetable not imported.");
                }
            });
    }

    fn show_pi_window(&mut self, ctx: &egui::Context) {
        egui::Window
            ::new("Performance Indices")
            .open(&mut self.show_pi_window)
            .show(ctx, |ui| {
                let performance_indices_shortest = self.performance_indices_shortest
                    .lock()
                    .unwrap();
                let performance_indices_optimized = self.performance_indices_optimized
                    .lock()
                    .unwrap();
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.show_pi_shortest, "shortest Path").clicked() {
                        self.show_pi_shortest = true;
                    }

                    if ui.selectable_label(!self.show_pi_shortest, "Optimized Path").clicked() {
                        self.show_pi_shortest = false;
                    }
                });
                ui.separator();
                ui.label("Performance Indices Overview");
                let pi_matrix = if self.show_pi_shortest {
                    performance_indices_shortest
                } else {
                    performance_indices_optimized
                };
                let hightlights = *self.current_period_iter.lock().unwrap();
                let optimization_status = self.optimization_status.lock().unwrap().clone();
                Grid::new("shortest_pi_grid")
                    .striped(true)
                    .num_columns(6)
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label("Monday");
                        ui.label("Tuesday");
                        ui.label("Wednesday");
                        ui.label("Thursday");
                        ui.label("Friday");
                        ui.end_row();
                        for index in 0..=11 {
                            ui.label(convert_periods(index));
                            for day in 1..=5 {
                                if
                                    (hightlights[(day - 1) as usize] as usize) == index &&
                                    optimization_status == OptimizationStatus::Calculating
                                {
                                    ui.label(
                                        RichText::new(
                                            pi_matrix[&day][&index].to_formatted_string(&Locale::fr)
                                        ).color(Color32::from_rgb(0xec, 0x6f, 0x27))
                                    );
                                } else {
                                    ui.label(
                                        pi_matrix[&day][&index].to_formatted_string(&Locale::fr)
                                    );
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.separator();
                ui.label("Current Period PI");
                ui.heading(
                    pi_matrix[&self.selected_day][&self.selected_period].to_formatted_string(
                        &Locale::fr
                    )
                );
                ui.separator();
                ui.label("Current Day PI");
                ui.heading(
                    pi_matrix[&self.selected_day]
                        .values()
                        .sum::<u128>()
                        .to_formatted_string(&Locale::fr)
                );
                ui.separator();
                ui.label("Total PI");
                ui.heading(
                    pi_matrix
                        .values()
                        .map(|day| day.values().sum::<u128>())
                        .sum::<u128>()
                        .to_formatted_string(&Locale::fr)
                );
                ui.separator();
                ui.label(
                    "Performance indices measure the overall performance of the routes. The lower the value, the better the performance."
                );
            });
    }
}

#[allow(dead_code)]
fn run_floyd_algorithm_cpp(
    filepath: PathBuf,
    path_generation_status_arc: Arc<Mutex<PathGenerationStatus>>,
    student_paths_arc: Arc<Mutex<Option<Routes>>>
) -> String {
    let bin_dir = fs::canonicalize("./bin").unwrap();
    if let Ok(binding) = fs::canonicalize(filepath) {
        let json_path = binding.to_str().unwrap();
        println!("{:?}\n{:?}", bin_dir, json_path);
        if
            let Ok(mut floyd_command) = Command::new("./floyd.out")
                .current_dir(&bin_dir)
                .stdin(Stdio::piped())
                .spawn()
        {
            floyd_command.stdin.as_mut().unwrap().write_all(json_path.as_bytes()).unwrap();
            let output = floyd_command.wait_with_output();
            if output.is_err() {
                *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
                    "Failed to execute algorithm binary [floyd.out].".to_owned()
                );
            } else if output.as_ref().unwrap().status.success() {
                *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::LoadingJSON;
                let result_path = bin_dir.join("routes.json");
                let Ok(file_content) = fs::read_to_string(result_path) else {
                    *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
                        "Failed to read result file [routes.json].".to_owned()
                    );
                    return "".to_owned();
                };
                let Ok(routes) = serde_json::from_str::<Routes>(&file_content) else {
                    *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
                        "Failed to parse result file [routes.json].".to_owned()
                    );
                    return "".to_owned();
                };
                *student_paths_arc.lock().unwrap() = Some(routes);
                *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Successful;
                return file_content;
            } else {
                *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
                    format!(
                        "Algorithm binary [floyd.out] exited with code {}.",
                        output.unwrap().status.code().unwrap()
                    )
                );
            }
        } else {
            *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
                "Failed to execute algorithm binary [floyd.out].".to_owned()
            );
        }
    } else {
        *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
            "Failed to find timetable file.".to_owned()
        );
    }
    "".to_owned()
}

#[allow(dead_code)]
fn run_floyd_algorithm_rust(
    filepath: PathBuf,
    shortest_paths_json: HashMap<String, String>,
    path_generation_status_arc: Arc<Mutex<PathGenerationStatus>>,
    student_paths_arc: Arc<Mutex<Option<Routes>>>
) {
    type Timetable = HashMap<String, HashMap<u32, HashMap<usize, String>>>;
    let mut result: Routes = HashMap::new();
    let Ok(timetable_file) = File::open(filepath) else {
        *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
            "Failed to find timetable file.".to_owned()
        );
        return;
    };
    let Ok(timetable_json) = serde_json::from_reader::<_, Timetable>(timetable_file) else {
        *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Failed(
            "Failed to parse timetable file. Please make sure the file is in JSON format.".to_owned()
        );
        return;
    };
    for student_number in timetable_json.keys() {
        let mut student_hashmap: HashMap<u32, HashMap<usize, String>> = HashMap::new();
        let mut student_timetable = timetable_json.get(student_number).unwrap().clone();
        for day in 1..=5 {
            let mut day_hashmap: HashMap<usize, String> = HashMap::new();
            student_timetable.entry(day).or_insert_with(HashMap::new);
            day_hashmap.insert(0, {
                let first_room = student_timetable.get(&day).unwrap().get(&1).unwrap().to_owned();
                if first_room == "G" {
                    "".to_owned()
                } else {
                    shortest_paths_json.get(format!("G{}", first_room).as_str()).unwrap().to_owned()
                }
            });
            for period in 1..=5 {
                day_hashmap.insert(period, {
                    let room_a = student_timetable
                        .get(&day)
                        .unwrap()
                        .get(&period)
                        .unwrap()
                        .to_owned();
                    let room_b = student_timetable
                        .get(&day)
                        .unwrap()
                        .get(&(period + 1))
                        .unwrap()
                        .to_owned();
                    if room_a == room_b {
                        "".to_owned()
                    } else {
                        shortest_paths_json
                            .get(format!("{}{}", room_a, room_b).as_str())
                            .unwrap()
                            .to_owned()
                    }
                });
            }
            day_hashmap.insert(6, {
                let first_room = student_timetable.get(&day).unwrap().get(&6).unwrap().to_owned();
                if first_room == "G" {
                    "".to_owned()
                } else {
                    shortest_paths_json.get(format!("{}G", first_room).as_str()).unwrap().to_owned()
                }
            });
            day_hashmap.insert(7, {
                let first_room = student_timetable.get(&day).unwrap().get(&7).unwrap().to_owned();
                if first_room == "G" {
                    "".to_owned()
                } else {
                    shortest_paths_json.get(format!("G{}", first_room).as_str()).unwrap().to_owned()
                }
            });
            for period in 7..=9 {
                day_hashmap.insert(period + 1, {
                    let room_a = student_timetable
                        .get(&day)
                        .unwrap()
                        .get(&period)
                        .unwrap()
                        .to_owned();
                    let room_b = student_timetable
                        .get(&day)
                        .unwrap()
                        .get(&(period + 1))
                        .unwrap()
                        .to_owned();
                    if room_a == room_b {
                        "".to_owned()
                    } else {
                        shortest_paths_json
                            .get(format!("{}{}", room_a, room_b).as_str())
                            .unwrap()
                            .to_owned()
                    }
                });
            }
            day_hashmap.insert(11, {
                let first_room = student_timetable.get(&day).unwrap().get(&10).unwrap().to_owned();
                if first_room == "G" {
                    "".to_owned()
                } else {
                    shortest_paths_json.get(format!("{}G", first_room).as_str()).unwrap().to_owned()
                }
            });
            student_hashmap.insert(day, day_hashmap);
        }
        result.insert(student_number.to_owned(), student_hashmap);
    }
    *student_paths_arc.lock().unwrap() = Some(result);
    *path_generation_status_arc.lock().unwrap() = PathGenerationStatus::Successful;
}

impl eframe::App for OptiWayApp {
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_validation_status = self.timetable_file_info.validation_status
            .lock()
            .unwrap()
            .clone();
        let current_congestion_status = self.congestion_status.lock().unwrap().clone();
        let current_path_status = self.path_generation_status.lock().unwrap().clone();
        let current_optimization_status = self.optimization_status.lock().unwrap().clone();
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label("OptiWay");
                egui::warn_if_debug_build(ui);
                ui.separator();
                if current_validation_status != TimetableValidationStatus::Successful {
                    ui.label(material_design_icons::MDI_CALENDAR_ALERT).on_hover_text(
                        "Timetable not imported."
                    );
                    ui.separator();
                }
                if self.selected_student.is_none() {
                    ui.label(material_design_icons::MDI_ACCOUNT_ALERT).on_hover_text(
                        "No student selected."
                    );
                    ui.separator();
                }
                if self.student_routes_shortest.lock().unwrap().is_none() {
                    ui.label(material_design_icons::MDI_SIGN_DIRECTION_REMOVE).on_hover_text(
                        "Path not calculated."
                    );
                    ui.separator();
                }
                if current_congestion_status != CongestionStatus::Successful {
                    ui.label(material_design_icons::MDI_VECTOR_POLYLINE_REMOVE).on_hover_text(
                        "Congestion not calculated."
                    );
                    ui.separator();
                }
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.show_json_validation {
                        ui.label("Validating timetable file");
                    } else if current_path_status.is_generating() {
                        ui.label("Calculating path");
                    } else if current_path_status.is_loading_json() {
                        ui.label("Loading paths");
                    } else if current_congestion_status.is_generating() {
                        ui.label("Evaluating congestion");
                    } else if current_optimization_status == OptimizationStatus::Calculating {
                        ui.label("Optimizing routes");
                    } else {
                        ui.label("Ready");
                    }
                });
            });
        });

        egui::SidePanel::right("side_panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.heading("Command center").on_hover_text(
                        "Follow the order of the buttons from top to bottom to complete the process."
                    );
                    if ui.button("Import timetable").clicked() {
                        let file = FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_directory("../")
                            .pick_file();
                        if let Some(file) = file {
                            self.timetable_file_info.filename = file
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_owned();
                            self.timetable_file_info.filepath = file;
                            self.show_json_validation = true;
                            *self.timetable_file_info.validation_status.lock().unwrap() =
                                TimetableValidationStatus::Ready;
                            self.selected_student = None;
                        }
                    }
                    ui.add_enabled_ui(
                        current_validation_status == TimetableValidationStatus::Successful,
                        |ui| {
                            if
                                ui
                                    .button("Baseline benchmark")
                                    .on_hover_text(
                                        "Generate shortest routes for all students as a baseline plan."
                                    )
                                    .on_disabled_hover_text("Import a timetable first.")
                                    .clicked()
                            {
                                self.show_path_gen_window = true;
                                *self.path_generation_status.lock().unwrap() =
                                    PathGenerationStatus::Ready;
                            }
                        }
                    );
                    ui.add_enabled_ui(
                        current_path_status == PathGenerationStatus::Successful,
                        |ui| {
                            if
                                ui
                                    .button("Calculate congestion")
                                    .on_disabled_hover_text("Calculate routes first.")
                                    .clicked()
                            {
                                self.show_congestion_window = true;
                                *self.congestion_status.lock().unwrap() = CongestionStatus::Ready;
                            }
                        }
                    );
                    ui.add_enabled_ui(
                        current_congestion_status == CongestionStatus::Successful,
                        |ui| {
                            if
                                ui
                                    .button("Optimize routes")
                                    .on_disabled_hover_text("Calculate congestion first.")
                                    .on_hover_text("Start the optimization iteration process.")
                                    .clicked()
                            {
                                self.show_optimization_window = true;
                                *self.optimization_status.lock().unwrap() =
                                    OptimizationStatus::ParamInput;
                            }
                        }
                    );
                    ui.separator();
                    ComboBox::from_label("Route display mode")
                        .selected_text(format!("{}", self.path_display))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.path_display,
                                PathDisplay::Shortest,
                                "Shortest route"
                            );
                            ui.add_enabled_ui(
                                self.student_routes_optimized.lock().unwrap().is_some(),
                                |ui| {
                                    ui.selectable_value(
                                        &mut self.path_display,
                                        PathDisplay::Optimized,
                                        "Optimized route"
                                    );
                                }
                            );
                        });
                    ComboBox::from_label("Student")
                        .selected_text(self.selected_student.clone().unwrap_or("—".to_owned()))
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit
                                    ::singleline(&mut self.student_number_search)
                                    .hint_text("Search")
                            );
                            ui.separator();
                            for student in self.student_list.lock().unwrap().iter() {
                                if student.contains(&self.student_number_search) {
                                    ui.selectable_value(
                                        &mut self.selected_student,
                                        Some(student.to_owned()),
                                        student
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
                                    convert_day_of_week(i)
                                );
                            }
                        });
                    ComboBox::from_label("Period")
                        .selected_text(convert_periods(self.selected_period))
                        .show_ui(ui, |ui| {
                            for i in 0..=11 {
                                ui.selectable_value(
                                    &mut self.selected_period,
                                    i,
                                    convert_periods(i)
                                );
                            }
                        });
                    ui.separator();
                    ui.heading("Panels");
                    if ui.button("Show path as text").clicked() {
                        self.show_path_window = true;
                    }
                    if ui.button("Show timetable").clicked() {
                        self.show_timetable_window = true;
                    }
                    if ui.button("Show performance indices").clicked() {
                        self.show_pi_window = true;
                    }
                    ui.separator();
                    ui.heading("Export");
                    if ui.button("Export shortest routes").clicked() {
                        let file = FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_directory("../")
                            .save_file();
                        if let Some(file) = file {
                            let mut file = File::create(file).unwrap();
                            let student_routes = self.student_routes_shortest.lock().unwrap();
                            let student_routes = student_routes.as_ref().unwrap();
                            let json = serde_json::to_string_pretty(&student_routes).unwrap();
                            file.write_all(json.as_bytes()).unwrap();
                        }
                    }
                    let enabled = self.student_routes_optimized.lock().unwrap().is_some();
                    ui.add_enabled_ui(enabled, |ui| {
                        if ui.button("Export optimized routes").clicked() {
                            let file = FileDialog::new()
                                .add_filter("JSON", &["json"])
                                .set_directory("../")
                                .save_file();
                            if let Some(file) = file {
                                let mut file = File::create(file).unwrap();
                                let student_routes = self.student_routes_optimized.lock().unwrap();
                                let student_routes = student_routes.as_ref().unwrap();
                                let json = serde_json::to_string_pretty(&student_routes).unwrap();
                                file.write_all(json.as_bytes()).unwrap();
                            }
                        }
                    });
                    ui.separator();
                    ui.heading("Floor view");
                    ui.horizontal(|ui| {
                        if ui.toggle_value(&mut self.selected_floor[0], "All").clicked() {
                            self.selected_floor_index = 0;
                            for i in 1..=8 {
                                self.selected_floor[i] = false;
                            }
                            if !self.selected_floor.contains(&true) {
                                self.selected_floor[0] = true;
                            }
                        }
                        for i in 2..=8 {
                            if
                                ui
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
                        Slider::new(&mut self.inactive_brightness, 32..=255).text(
                            "Inactive floor brightness"
                        )
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.selectable_label(!self.show_congestion, "Show paths").clicked() {
                            self.show_congestion = false;
                        }
                        if ui.selectable_label(self.show_congestion, "Show congestion").clicked() {
                            self.show_congestion = true;
                        }
                    });
                    if self.show_congestion {
                        ui.checkbox(&mut self.show_congestion_path, "Show path congestion");
                        ui.checkbox(&mut self.show_congestion_point, "Show node congestion");
                        ui.heading("Legend");
                        egui::Grid
                            ::new("congestion_legend")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label(RichText::new("●").color(congestion_color_scale(0)));
                                ui.label("No students");
                                ui.end_row();
                                ui.label(RichText::new("●").color(congestion_color_scale(1)));
                                ui.label("1-20 students");
                                ui.end_row();
                                ui.label(RichText::new("●").color(congestion_color_scale(21)));
                                ui.label("21–50 students");
                                ui.end_row();
                                ui.label(RichText::new("●").color(congestion_color_scale(51)));
                                ui.label("51–100 students");
                                ui.end_row();
                                ui.label(RichText::new("●").color(congestion_color_scale(101)));
                                ui.label("101–200 students");
                                ui.end_row();
                                ui.label(RichText::new("●").color(congestion_color_scale(201)));
                                ui.label("201–400 students");
                                ui.end_row();
                                ui.label(RichText::new("●").color(congestion_color_scale(401)));
                                ui.label("≥401 students");
                                ui.end_row();
                            });
                        ui.add(
                            Slider::new(&mut self.congestion_filter, 0..=400).text(
                                "Minimum congestion"
                            )
                        );
                    } else {
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
                                color_picker::Alpha::Opaque
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
                                color_picker::Alpha::Opaque
                            );
                        });
                    }
                });
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            if self.show_json_validation {
                self.show_json_validation_window(ctx, current_validation_status);
            }

            if self.show_path_gen_window {
                self.show_path_generation_window(ctx, current_path_status);
            }

            if self.show_timetable_window {
                self.show_timetable_window(ctx);
            }

            if self.show_congestion_window {
                self.show_congestion_window(ctx, current_congestion_status);
            }
            if self.show_pi_window {
                self.show_pi_window(ctx);
            }
            if self.show_optimization_window {
                self.show_optimization_window(ctx, current_optimization_status);
            }

            // Paths

            let path_list: Vec<String> = if self.selected_student.is_some() {
                let student_number = self.selected_student.clone().unwrap();
                let student_routes = if self.path_display == PathDisplay::Optimized {
                    self.student_routes_optimized.lock().unwrap().clone()
                } else {
                    self.student_routes_shortest.lock().unwrap().clone()
                };
                if let Some(student_routes) = student_routes {
                    student_routes[&student_number][&self.selected_day][&self.selected_period]
                        .split(' ')
                        .collect::<Vec<&str>>()
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect::<Vec<String>>()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let mut segments: Vec<&[i32; 3]> = vec![];

            for i in &path_list {
                if self.projection_coords.contains_key(i) {
                    segments.push(&self.projection_coords[i]);
                }
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
                        load_image_from_path(
                            Path::new(
                                format!(
                                    "../assets/projection-transparent/projection_{i}F.png"
                                ).as_str()
                            )
                        ).unwrap(),
                        Default::default()
                    )
                });
                textures.push(texture_cur.clone());
            }
            let desired_size = ui.available_size_before_wrap();
            if desired_size.y < (desired_size.x / 2243.0) * (1221.0 + 350.0) {
                ui.label("▲ There may not be enough space to display the floor plan.");
            }
            let (_id, rect) = ui.allocate_space(desired_size);
            let scale = rect.width() / 2243.0;

            // Paint floor projections

            let current_floor_z = if self.selected_floor_index == 0 {
                0
            } else {
                ((self.selected_floor_index - 2) * 50) as i32
            };

            if !self.show_congestion {
                for (i, point) in segments.iter().enumerate() {
                    if i != 0 {
                        ui.painter().circle_filled(convert_pos(&rect, point, scale), 4.0, if
                            self.selected_floor_index == 0 ||
                            (current_floor_z >= point[2].min(segments[i - 1][2]) &&
                                current_floor_z <= point[2].max(segments[i - 1][2]))
                        {
                            self.active_path_color
                        } else {
                            self.inactive_path_color
                        });
                        ui.painter().line_segment(
                            [
                                convert_pos(&rect, segments[i - 1], scale),
                                convert_pos(&rect, point, scale),
                            ],
                            if
                                self.selected_floor_index == 0 ||
                                (current_floor_z >= point[2].min(segments[i - 1][2]) &&
                                    current_floor_z <= point[2].max(segments[i - 1][2]))
                            {
                                Stroke::new(4.0, self.active_path_color)
                            } else {
                                Stroke::new(4.0, self.inactive_path_color)
                            }
                        );
                    } else {
                        ui.painter().circle_filled(convert_pos(&rect, point, scale), 4.0, if
                            self.selected_floor_index == 0 ||
                            current_floor_z == point[2].min(segments[i][2])
                        {
                            self.active_path_color
                        } else {
                            self.inactive_path_color
                        });
                    }
                }
            } else {
                if self.show_congestion_path {
                    if
                        self.path_display == PathDisplay::Optimized &&
                        self.student_routes_optimized.lock().unwrap().is_some()
                    {
                        for ((node1, node2), congestion) in self.congestion_path_data_opt
                            .lock()
                            .unwrap()
                            .clone()
                            .get(&self.selected_day)
                            .unwrap()
                            .get(&self.selected_period)
                            .unwrap() {
                            if node1 == "G" || node2 == "G" || congestion < &self.congestion_filter {
                                continue;
                            }
                            let node1_pos = self.projection_coords[node1];
                            let node2_pos = self.projection_coords[node2];
                            if
                                self.selected_floor_index == 0 ||
                                (current_floor_z >= node1_pos[2].min(node2_pos[2]) &&
                                    current_floor_z <= node1_pos[2].max(node2_pos[2]))
                            {
                                ui.painter().line_segment(
                                    [
                                        convert_pos(&rect, &node1_pos, scale),
                                        convert_pos(&rect, &node2_pos, scale),
                                    ],
                                    Stroke::new(4.0, congestion_color_scale(*congestion))
                                );
                            }
                        }
                    } else {
                        for ((node1, node2), congestion) in self.congestion_path_data
                            .lock()
                            .unwrap()
                            .clone()
                            .get(&self.selected_day)
                            .unwrap()
                            .get(&self.selected_period)
                            .unwrap() {
                            if node1 == "G" || node2 == "G" || congestion < &self.congestion_filter {
                                continue;
                            }
                            let node1_pos = self.projection_coords[node1];
                            let node2_pos = self.projection_coords[node2];
                            if
                                self.selected_floor_index == 0 ||
                                (current_floor_z >= node1_pos[2].min(node2_pos[2]) &&
                                    current_floor_z <= node1_pos[2].max(node2_pos[2]))
                            {
                                ui.painter().line_segment(
                                    [
                                        convert_pos(&rect, &node1_pos, scale),
                                        convert_pos(&rect, &node2_pos, scale),
                                    ],
                                    Stroke::new(4.0, congestion_color_scale(*congestion))
                                );
                            }
                        }
                    }
                }
                if self.show_congestion_point {
                    if
                        self.path_display == PathDisplay::Optimized &&
                        self.student_routes_optimized.lock().unwrap().is_some()
                    {
                        for (room, congestion) in self.congestion_point_data
                            .lock()
                            .unwrap()
                            .clone()
                            .get(&self.selected_day)
                            .unwrap()
                            .get(&self.selected_period)
                            .unwrap() {
                            if room == "G" || room.is_empty() {
                                continue;
                            }
                            let coords = self.projection_coords.get(room).unwrap();
                            if
                                (self.selected_floor_index == 0 || current_floor_z == coords[2]) &&
                                *congestion >= self.congestion_filter
                            {
                                ui.painter().circle_filled(
                                    convert_pos(&rect, coords, scale),
                                    4.0,
                                    congestion_color_scale(*congestion)
                                );
                            }
                        }
                    } else {
                        for (room, congestion) in self.congestion_point_data_opt
                            .lock()
                            .unwrap()
                            .clone()
                            .get(&self.selected_day)
                            .unwrap()
                            .get(&self.selected_period)
                            .unwrap() {
                            if room == "G" || room.is_empty() {
                                continue;
                            }
                            let coords = self.projection_coords.get(room).unwrap();
                            if
                                (self.selected_floor_index == 0 || current_floor_z == coords[2]) &&
                                *congestion >= self.congestion_filter
                            {
                                ui.painter().circle_filled(
                                    convert_pos(&rect, coords, scale),
                                    4.0,
                                    congestion_color_scale(*congestion)
                                );
                            }
                        }
                    }
                }
            }

            for (i, texture) in textures.iter().enumerate().take(7) {
                let rect = Rect::from_min_size(
                    rect.min,
                    emath::vec2(rect.width(), rect.width() / texture.aspect_ratio())
                ).translate(emath::vec2(0.0, ((7 - i) as f32) * 50.0 * scale));

                ui.painter().image(
                    texture.into(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    if self.selected_floor[0] || self.selected_floor[i + 2] {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(self.inactive_brightness)
                    }
                );
            }
            // Special case: the floor is selected, so needs to be repainted last
            if self.selected_floor_index != 0 {
                let texture = textures[self.selected_floor_index - 2].clone();
                let rect = Rect::from_min_size(
                    rect.min,
                    emath::vec2(rect.width(), rect.width() / texture.aspect_ratio())
                ).translate(
                    emath::vec2(0.0, ((7 - (self.selected_floor_index - 2)) as f32) * 50.0 * scale)
                );

                ui.painter().image(
                    (&texture).into(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE
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

    (
        emath::vec2(
            rect.left() + 25.0 * ANGLE.cos() * scale,
            rect.top() + (50.0 + 350.0 + 25.0 * ANGLE.sin()) * scale
        ) +
        emath::vec2(
            ((pos[0] as f32) * ANGLE.cos() + (pos[1] as f32) * ANGLE.cos()) * scale,
            ((pos[0] as f32) * ANGLE.sin() - (pos[1] as f32) * ANGLE.sin() - (pos[2] as f32)) *
                scale
        )
    ).to_pos2()
}

fn convert_day_of_week(day: u32) -> String {
    (
        match day {
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            _ => "Unknown",
        }
    ).into()
}

fn convert_periods(index: usize) -> String {
    (
        match index {
            0 => "Before P1",
            1 => "P1–P2",
            2 => "P2–P3",
            3 => "P3–P4",
            4 => "P4–P5",
            5 => "P5–P6",
            6 => "P6–Lunch",
            7 => "Lunch–P7",
            8 => "P7–P8",
            9 => "P8–P9",
            10 => "P9–P10",
            11 => "After P10",
            _ => "Unknown",
        }
    ).into()
}
fn congestion_color_scale(congestion: u32) -> Color32 {
    match congestion {
        0 => Color32::from_rgb(0x61, 0x61, 0x61),
        1..=20 => Color32::from_rgb(0x00, 0x7a, 0xf5),
        21..=50 => Color32::from_rgb(0x14, 0xae, 0x52),
        51..=100 => Color32::from_rgb(0xff, 0xc1, 0x07),
        101..=200 => Color32::from_rgb(0xec, 0x6f, 0x27),
        201..=400 => Color32::from_rgb(0xe4, 0x37, 0x48),
        _ => Color32::from_rgb(0x91, 0x54, 0xff),
    }
}

fn congestion_range_index(congestion: u32) -> usize {
    match congestion {
        0 => 0,
        1..=20 => 1,
        21..=50 => 2,
        51..=100 => 3,
        101..=200 => 4,
        201..=400 => 5,
        _ => 6,
    }
}

#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "OptiWay",
        native_options,
        Box::new(|cc| {
            // egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(optiway::OptiWayApp::new(cc))
        })
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner
            ::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| Box::new(optiway::OptiWayApp::new(cc)))
            ).await
            .expect("failed to start eframe");
    });
}

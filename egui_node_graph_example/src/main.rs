#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

use eframe::egui::Visuals;
// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eframe::run_native(
        "Egui node graph example",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(Visuals::dark());
            #[cfg(feature = "persistence")]
            {
                Box::new(egui_node_graph_example::NodeGraphExample::new(cc))
            }
            #[cfg(not(feature = "persistence"))]
            Box::new(egui_node_graph_example::NodeGraphExample::default())
        }),
    );
}

// when compiling to web using trunk.

#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| {
                cc.egui_ctx.set_visuals(Visuals::dark());
                #[cfg(feature = "persistence")]
                {
                    Box::new(egui_node_graph_example::NodeGraphExample::new(cc))
                }
                #[cfg(not(feature = "persistence"))]
                Box::new(egui_node_graph_example::NodeGraphExample::default())
            }),
        )
        .await
        .expect("failed to start eframe");
    });
}

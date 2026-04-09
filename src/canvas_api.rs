//! Canvas API abstraction layer for web and desktop
//! 
//! This module provides a platform-agnostic API for canvas operations.
//! On web targets, it uses WebAssembly and JavaScript.
//! On desktop targets, it's a no-op (operations are disabled).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedrawConfig {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub drawed_rois: Vec<Vec<(i32, i32)>>,
    pub current_points: Vec<(i32, i32)>,
    pub mouse_xy: Option<(f64, f64)>,
    pub highlight: Option<HighlightConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightConfig {
    pub name: String,
    pub is_edit: bool,
}

#[cfg(feature = "web")]
pub use web_impl::redraw_canvas;

#[cfg(feature = "web")]
mod web_impl {
    use super::*;
    use dioxus::logger::tracing;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = "drawROICanvas")]
        fn draw_roi_canvas_js(config: &JsValue);
    }

    pub fn redraw_canvas(
        _canvas_id: &str,
        config: &RedrawConfig,
        _image_id: Option<&str>,
    ) {
        if let Ok(config_value) = serde_wasm_bindgen::to_value(config) {
            draw_roi_canvas_js(&config_value);
        } else {
            tracing::error!("Failed to serialize redraw config");
        }
    }
}

#[cfg(not(feature = "web"))]
pub fn redraw_canvas(
    _canvas_id: &str,
    _config: &RedrawConfig,
    _image_id: Option<&str>,
) {
    // No-op for non-web targets
}

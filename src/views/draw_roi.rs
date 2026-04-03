use dioxus::{logger::tracing, prelude::*, web::WebEventExt};
use web_sys::{
    wasm_bindgen::JsCast, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement,
};
use base64::prelude::*;
use crate::components::{aspect_ratio::AspectRatio, input::Input};

#[derive(Clone, Copy)]
struct ViewState {
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

#[component]
pub fn DrawRoiPage() -> Element {
    let mut selected_file = use_signal(|| String::new());
    let mut image_ref = use_signal(|| None::<HtmlImageElement>);
    let mut canvas_ref = use_signal(|| None::<HtmlCanvasElement>);

    // use_effect(move || {
    //     if selected_file().is_empty() {
    //         return;
    //     }
    //     if let Some(image) = image_ref() {
    //         tracing::info!("image.set_src");
    //         image.set_src(&selected_file());
    //     }
    // });

    let on_file_input = move |e: FormEvent| async move {
        let files = e.files().clone();
        tracing::info!("Input files {:?}", files);
        let Some(file) = files.get(0) else {
            return;
        };
        let Ok(image_data) = file.read_bytes().await else {
            return;
        };
        let img_str = String::from("data:image/jpeg;base64,") + BASE64_STANDARD.encode(image_data).as_str();
        selected_file.set(img_str);
    };

    let on_image_load = move |_| {
        tracing::info!("on_image_load");
        if let (Some(canvas), Some(image)) = (canvas_ref(), image_ref()) {
            let ctx = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();

            let scale_value = 1.0;
            let width = image.width() as f64 * scale_value;
            let height = image.height() as f64 * scale_value;

            ctx.draw_image_with_html_image_element_and_dw_and_dh(&image, 0.0, 0.0, width, height)
                .unwrap();
            tracing::info!("Draw image!");
        }
    };

    rsx! {
        img {
            class: "hidden",
            src: selected_file(),
            onload: on_image_load,
            onmounted: move |e| {
                tracing::info!("img.onmounted");
                image_ref
                    .set(Some(e.as_web_event().dyn_into::<HtmlImageElement>().unwrap().clone()))

            },
        }

        div { class: "grid grid-cols-1 gap-2",

            Input { r#type: "file", accept: "image/*", oninput: on_file_input }
            AspectRatio { ratio: 16. / 9.,

                canvas {
                    class: "w-full h-full border",
                    width: 1920,
                    height: 1080,
                    onmounted: move |e| {
                        tracing::info!("canvas.onmounted");
                        canvas_ref
                            .set(Some(e.as_web_event().dyn_into::<HtmlCanvasElement>().unwrap().clone()))
                    },
                }
            
            }
        
        }

    }
}

// #[component]
// fn DrawRoiPage() -> Element {
//     let canvas_ref = use_signal(|| None::<HtmlCanvasElement>);
//     let view = use_signal(|| ViewState {
//         scale: 1.0,
//         offset_x: 0.0,
//         offset_y: 0.0,
//     });
//     let is_panning = use_signal(|| false);
//     let last_pos = use_signal(|| (0.0, 0.0));

//     // 縮放：滑鼠滾輪 / 手機雙指捏合
//     let on_wheel = move |e: Event<WheelData>| {
//         e.prevent_default();
//         let delta = e.data.delta_y();
//         let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };

//         view.update(|v| {
//             v.scale = (v.scale * zoom_factor).clamp(0.1, 5.0);
//         });
//     };

//     // 觸控：單指拖曳平移，雙指捏合縮放
//     let on_touch_start = move |e: Event<TouchData>| {
//         let touches = e.data.touches();
//         match touches.len() {
//             1 => {
//                 // 單指：開始平移
//                 let touch = touches.get(0).unwrap();
//                 is_panning.set(true);
//                 last_pos.set((touch.client_x() as f64, touch.client_y() as f64));
//             }
//             2 => { // 雙指：記錄初始距離（用於捏合縮放）
//                  // 計算兩指距離，存起來...
//             }
//             _ => {}
//         }
//     };

//     // 滑鼠：右鍵或中鍵拖曳平移，左鍵畫圖
//     // 觸控：單指畫圖，雙指平移（或用按鈕切換模式）

//     rsx! {
//         div { class: "toolbar",
//             button { onclick: move |_| view.set(ViewState { scale: 1.0, offset_x: 0.0, offset_y: 0.0 }),
//                 "重置視圖"
//             }
//             span { "縮放: {view().scale:.1}x" }
//         }

//         canvas {
//             width: "800",
//             height: "600",
//             onwheel: on_wheel,
//             ontouchstart: on_touch_start,
//             ontouchmove: ...,  // 處理捏合或拖曳
//             ontouchend: move |_| is_panning.set(false),
//             // ... 滑鼠事件
//         }
//     }
// }

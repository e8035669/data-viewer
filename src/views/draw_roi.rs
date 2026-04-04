use crate::components::{aspect_ratio::AspectRatio, input::Input};
use base64::prelude::*;
use dioxus::{html::input_data::MouseButton, logger::tracing, prelude::*, web::WebEventExt};
use web_sys::{
    wasm_bindgen::JsCast, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement,
};

#[derive(Clone, Copy)]
struct ViewState {
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

#[component]
pub fn DrawRoiPage() -> Element {
    let mut selected_file = use_signal(|| String::new());
    let mut canvas_ref = use_signal(|| None::<HtmlCanvasElement>);
    let mut canvas_ctx = use_signal(|| None::<CanvasRenderingContext2d>);
    let mut draw_ctx = use_signal(|| DrawContext {
        image: None,
        canvas_height: 1080.,
        canvas_width: 1920.,
        display_height: 1080.,
        display_width: 1920.,
        scale: 1.0,
        offset_x: 0.0,
        offset_y: 0.0,
        mouse_display_xy: None,
        mouse_canvas_xy: None,
    });

    use_effect(move || {
        let Some(ctx) = canvas_ctx() else {
            return;
        };
        redraw(&ctx, &draw_ctx());
    });

    let on_file_input = move |e: FormEvent| async move {
        let files = e.files().clone();
        tracing::info!("Input files {:?}", files);
        let Some(file) = files.get(0) else {
            return;
        };
        let Ok(image_data) = file.read_bytes().await else {
            return;
        };
        let img_str =
            String::from("data:image/jpeg;base64,") + BASE64_STANDARD.encode(image_data).as_str();
        selected_file.set(img_str);
    };

    let on_image_load = move |_| {
        draw_ctx.write();
    };

    // 縮放：滑鼠滾輪 / 手機雙指捏合
    let on_wheel = move |e: Event<WheelData>| {
        e.prevent_default();
        let delta = e.delta().strip_units().y;
        let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };

        {
            let mut ctx = draw_ctx.write();
            ctx.scale = (ctx.scale * zoom_factor).clamp(0.1, 5.0);
            tracing::info!("New scale: {}", ctx.scale);

            if let Some(xy) = &ctx.mouse_canvas_xy {
                // TODO: 以滑鼠為中心放大
            }
        }
    };

    let on_mouse_move = move |e: MouseEvent| {
        e.prevent_default();
        let mut ctx = draw_ctx.write();
        let xy = e.element_coordinates();

        ctx.mouse_display_xy = Some((xy.x, xy.y));
        ctx.mouse_canvas_xy = Some(ctx.to_canvas_pos(xy.x, xy.y));
        tracing::info!(
            "Move xy display: {:?}, canvas: {:?}",
            ctx.mouse_display_xy,
            ctx.mouse_canvas_xy
        );

        if e.held_buttons().contains(MouseButton::Primary) {
            let we = e.as_web_event();

            let move_canvas_xy = ctx.to_canvas_pos(we.movement_x() as f64, we.movement_y() as f64);
            ctx.offset_x += move_canvas_xy.0;
            ctx.offset_y += move_canvas_xy.1;
        }
    };

    let on_mouse_leave = move |e: MouseEvent| {
        e.prevent_default();
        let mut ctx = draw_ctx.write();
        ctx.mouse_canvas_xy = None;
        ctx.mouse_display_xy = None;
    };

    rsx! {
        img {
            class: "hidden",
            src: selected_file(),
            onload: on_image_load,
            onmounted: move |e| {
                tracing::info!("img.onmounted");
                let image_elem = e
                    .as_web_event()
                    .dyn_into::<HtmlImageElement>()
                    .unwrap()
                    .clone();
                draw_ctx.write().image = Some(image_elem);
            },
        }

        div { class: "grid grid-cols-1 gap-2",

            Input { r#type: "file", accept: "image/*", oninput: on_file_input }

            AspectRatio { ratio: 16. / 9.,
                canvas {
                    class: "w-full h-full border",
                    width: 1920,
                    height: 1080,
                    onwheel: on_wheel,
                    onmousemove: on_mouse_move,
                    onmouseleave: on_mouse_leave,
                    onresize: move |e| {
                        e.prevent_default();
                        tracing::info!("canvas.onresize {:?}", e.data());
                        {
                            let mut ctx = draw_ctx.write();
                            let s = e.get_content_box_size().unwrap();
                            ctx.display_height = s.height;
                            ctx.display_width = s.width;
                        }
                    },
                    onmounted: move |e| {
                        tracing::info!("canvas.onmounted");

                        canvas_ref
                            .set(
                                Some(e.as_web_event().dyn_into::<HtmlCanvasElement>().unwrap().clone()),
                            );
                        let ctx = canvas_ref()
                            .unwrap()
                            .get_context("2d")
                            .unwrap()
                            .unwrap()
                            .dyn_into::<CanvasRenderingContext2d>()
                            .unwrap();
                        canvas_ctx.set(Some(ctx));
                    },
                }
            }
        }
    }
}

#[derive(Clone)]
struct DrawContext {
    image: Option<HtmlImageElement>,
    canvas_height: f64,
    canvas_width: f64,
    display_height: f64,
    display_width: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    mouse_display_xy: Option<(f64, f64)>,
    mouse_canvas_xy: Option<(f64, f64)>,
}

impl DrawContext {
    fn to_canvas_pos(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.canvas_width / self.display_width / self.scale,
            y * self.canvas_height / self.display_height / self.scale,
        )
    }

}

fn redraw(ctx: &CanvasRenderingContext2d, draw_ctx: &DrawContext) {
    ctx.reset();
    ctx.scale(draw_ctx.scale, draw_ctx.scale).unwrap();

    if let Some(image) = &draw_ctx.image {
        ctx.draw_image_with_html_image_element(image, draw_ctx.offset_x, draw_ctx.offset_y)
            .unwrap();
    }

    if let Some(xy) = &draw_ctx.mouse_canvas_xy {
        ctx.set_line_width(5.0);
        ctx.begin_path();
        ctx.move_to(xy.0 - 10., xy.1);
        ctx.line_to(xy.0 + 10., xy.1);
        ctx.move_to(xy.0, xy.1 - 10.);
        ctx.line_to(xy.0, xy.1 + 10.);
        ctx.stroke();
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

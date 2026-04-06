use crate::components::{
    aspect_ratio::AspectRatio,
    button::{Button, ButtonVariant},
    input::Input,
    scroll_area::ScrollArea,
    toggle_group::{ToggleGroup, ToggleItem},
    toolbar::{Toolbar, ToolbarButton, ToolbarGroup},
};
use base64::prelude::*;
use dioxus::{html::input_data::MouseButton, logger::tracing, prelude::*, web::WebEventExt};
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::scroll_area::ScrollDirection;
use euclid::{point2, size2, Point2D};
use indexmap::IndexMap;
use web_sys::{
    wasm_bindgen::JsCast, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement,
};

#[derive(Default, Debug, Clone, Copy)]
enum ToolMode {
    #[default]
    View,
    Draw,
    Edit,
    Delete,
}

#[derive(Default, Debug, Clone)]
struct ToolStatus {
    mode: ToolMode,
    highlight: Option<String>,
    editing: Option<String>,
}

struct Pixel;

#[derive(Debug, Clone)]
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
    drawed_rois: IndexMap<String, Vec<Point2D<i32, Pixel>>>,
    current_points: Vec<Point2D<i32, Pixel>>,
    mouse_down_pos: Option<(f64, f64)>,
    is_dragging: bool,
}

impl DrawContext {
    fn new() -> Self {
        Self {
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
            current_points: Vec::new(),
            drawed_rois: IndexMap::new(),
            mouse_down_pos: None,
            is_dragging: false,
        }
    }

    fn to_canvas_pos(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.canvas_width / self.display_width / self.scale,
            y * self.canvas_height / self.display_height / self.scale,
        )
    }

    fn canvas_resize(&mut self, e: &ResizeEvent) {
        let s = e.get_content_box_size().unwrap();
        self.display_height = s.height;
        self.display_width = s.width;
    }

    fn redraw(&self, ctx: &CanvasRenderingContext2d) {
        DrawContext::redraw0(self, ctx);
    }

    fn redraw0(draw_ctx: &DrawContext, ctx: &CanvasRenderingContext2d) {
        ctx.reset();
        ctx.scale(draw_ctx.scale, draw_ctx.scale).unwrap();

        if let Some(image) = &draw_ctx.image {
            ctx.draw_image_with_html_image_element(image, draw_ctx.offset_x, draw_ctx.offset_y)
                .unwrap();
        }

        let offset_xy = size2(draw_ctx.offset_x, draw_ctx.offset_y);

        // 繪製已完成的多邊形（綠色）
        ctx.set_stroke_style_str("green");
        ctx.set_fill_style_str("green");
        ctx.set_line_width(2.0);
        for roi in draw_ctx.drawed_rois.values() {
            ctx.begin_path();
            for (i, p) in roi.iter().enumerate() {
                let canvas_xy = p.to_f64().add_size(&offset_xy);
                if i == 0 {
                    ctx.move_to(canvas_xy.x, canvas_xy.y);
                } else {
                    ctx.line_to(canvas_xy.x, canvas_xy.y);
                }
            }
            ctx.close_path();
            ctx.stroke();

            // 繪製已完成多邊形的頂點（小圓點）
            for p in roi.iter() {
                let canvas_xy = p.to_f64().add_size(&offset_xy);
                ctx.begin_path();
                ctx.arc(
                    canvas_xy.x,
                    canvas_xy.y,
                    4.0,
                    0.0,
                    std::f64::consts::PI * 2.0,
                )
                .unwrap();
                ctx.fill();
            }
        }

        // 繪製目前多邊形（虛線效果用線寬變化）
        ctx.set_stroke_style_str("red");
        ctx.set_fill_style_str("red");
        ctx.set_line_width(2.0);
        if !draw_ctx.current_points.is_empty() {
            ctx.begin_path();
            for (i, p) in draw_ctx.current_points.iter().enumerate() {
                let canvas_xy = p.to_f64().add_size(&offset_xy);
                if i == 0 {
                    ctx.move_to(canvas_xy.x, canvas_xy.y);
                } else {
                    ctx.line_to(canvas_xy.x, canvas_xy.y);
                }
            }
            // 繪製從最後一個點到滑鼠位置的預覽線
            if let Some(xy) = draw_ctx.mouse_canvas_xy {
                ctx.line_to(xy.0, xy.1);
            }
            ctx.stroke();

            // 繪製目前多邊形的頂點（較大的圓點）
            for p in draw_ctx.current_points.iter() {
                let canvas_xy = p.to_f64().add_size(&offset_xy);
                ctx.begin_path();
                ctx.arc(
                    canvas_xy.x,
                    canvas_xy.y,
                    5.0,
                    0.0,
                    std::f64::consts::PI * 2.0,
                )
                .unwrap();
                ctx.fill();
            }
        }

        // 繪製滑鼠十字準線
        ctx.set_stroke_style_str("black");
        ctx.set_fill_style_str("black");
        if let Some(xy) = &draw_ctx.mouse_canvas_xy {
            ctx.set_line_width(1.0);
            ctx.begin_path();
            ctx.move_to(xy.0 - 10., xy.1);
            ctx.line_to(xy.0 + 10., xy.1);
            ctx.move_to(xy.0, xy.1 - 10.);
            ctx.line_to(xy.0, xy.1 + 10.);
            ctx.stroke();
        }
    }

    fn canvas_wheel(&mut self, delta: f64) {
        let zoom_factor = if delta > 0.0 { 0.9 } else { 1.1 };

        self.scale = (self.scale * zoom_factor).clamp(0.1, 5.0);
        tracing::info!("New scale: {}", self.scale);

        if let (Some(xy), Some(can_xy)) = (&self.mouse_display_xy, &self.mouse_canvas_xy) {
            let image_xy = (can_xy.0 - self.offset_x, can_xy.1 - self.offset_y);
            let new_canvas_xy = self.to_canvas_pos(xy.0, xy.1);
            let new_offset_xy = (new_canvas_xy.0 - image_xy.0, new_canvas_xy.1 - image_xy.1);
            self.mouse_canvas_xy = Some(new_canvas_xy);
            self.offset_x = new_offset_xy.0;
            self.offset_y = new_offset_xy.1;
        }
    }
}

#[derive(Debug, Clone, Store)]
struct DrawRoiContext {
    tool_ctx: ToolStatus,
    draw_ctx: DrawContext,
    canvas_ctx: Option<CanvasRenderingContext2d>,
    canvas_ref: Option<HtmlCanvasElement>,
}

impl DrawRoiContext {
    fn new() -> Self {
        Self {
            tool_ctx: Default::default(),
            draw_ctx: DrawContext::new(),
            canvas_ctx: None,
            canvas_ref: None,
        }
    }

    fn canvas_mounted(&mut self, e: &MountedEvent) {
        let canvas_ref = e.as_web_event().dyn_into::<HtmlCanvasElement>().unwrap();
        let ctx = canvas_ref
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        self.canvas_ref = Some(canvas_ref);
        self.canvas_ctx = Some(ctx);
    }

    fn canvas_move(&mut self, e: &MouseEvent) {
        let ctx = &mut self.draw_ctx;
        let xy = e.element_coordinates();

        ctx.mouse_display_xy = Some((xy.x, xy.y));
        ctx.mouse_canvas_xy = Some(ctx.to_canvas_pos(xy.x, xy.y));

        if e.held_buttons().contains(MouseButton::Primary) {
            // 偵測是否發生了足夠的拖曳（超過 5 像素閾值）
            if let Some((down_x, down_y)) = ctx.mouse_down_pos {
                let dx = xy.x - down_x;
                let dy = xy.y - down_y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance > 5.0 {
                    ctx.is_dragging = true;
                }
            }

            let we = e.as_web_event();
            let move_canvas_xy = ctx.to_canvas_pos(we.movement_x() as f64, we.movement_y() as f64);
            ctx.offset_x += move_canvas_xy.0;
            ctx.offset_y += move_canvas_xy.1;
        }
    }

    fn canvas_leave(&mut self, e: &MouseEvent) {
        let ctx = &mut self.draw_ctx;
        ctx.mouse_canvas_xy = None;
        ctx.mouse_display_xy = None;
    }

    fn canvas_mouse_down(&mut self, e: &MouseEvent) {
        let xy = e.element_coordinates();
        let ctx = &mut self.draw_ctx;
        ctx.mouse_down_pos = Some((xy.x, xy.y));
        ctx.is_dragging = false;
        tracing::info!("on_mouse_down at {:?}", ctx.mouse_down_pos);
    }

    fn canvas_click(&mut self, e: &MouseEvent) {
        let ctx = &mut self.draw_ctx;
        let js_event = e
            .as_web_event()
            .dyn_into::<web_sys::PointerEvent>()
            .unwrap();
        tracing::info!("on_click {:?}", js_event);

        // 只有在沒有拖曳時才認為是有效的點擊
        if ctx.is_dragging {
            tracing::info!("忽略拖曳後的點擊");
            ctx.mouse_down_pos = None;
            return;
        }

        if let Some(canvas_xy) = ctx.mouse_canvas_xy {
            let offset = size2(-ctx.offset_x, -ctx.offset_y);
            let point = point2(canvas_xy.0, canvas_xy.1).add_size(&offset).to_i32();
            ctx.current_points.push(point);
            tracing::info!("新增點：{:?}", point);
        }
        ctx.mouse_down_pos = None;
    }

    fn canvas_double_click(&mut self, _e: &MouseEvent) {
        let ctx = &mut self.draw_ctx;
        // 雙擊之前會有兩次單擊，要刪掉
        ctx.current_points.pop();
        ctx.current_points.pop();

        if ctx.current_points.len() > 2 {
            // 閉合目前多邊形並新增至已完成清單
            let completed_roi = ctx.current_points.clone();
            let roi_name = format!("ROI {}", ctx.drawed_rois.len()); // 自動生成名稱
            ctx.drawed_rois.insert(roi_name, completed_roi);
            ctx.current_points.clear();
            tracing::info!("已閉合多邊形，共有 ROI 數：{}", ctx.drawed_rois.len());
        } else {
            tracing::info!("無法閉合多邊形 - 需要至少 3 個點");
        }
        ctx.mouse_down_pos = None;
        ctx.is_dragging = false;
    }
}

fn roi_to_string(roi: &Vec<Point2D<i32, Pixel>>) -> String {
    let mut ret = String::from("[");
    let content = roi
        .iter()
        .map(|p| format!("\'{},{}\'", p.x, p.y))
        .collect::<Vec<_>>()
        .join(",");
    ret.push_str(&content);
    ret.push(']');
    ret
}

fn rois_to_string(rois: &IndexMap<String, Vec<Point2D<i32, Pixel>>>) -> String {
    let mut ret = String::from("[");
    let content = rois
        .values()
        .map(roi_to_string)
        .collect::<Vec<_>>()
        .join(",");
    ret.push_str(&content);
    ret.push(']');
    ret
}

#[component]
pub fn DrawRoiPage() -> Element {
    let mut selected_file = use_signal(|| String::new());

    let mut draw_roi_ctx = use_store(|| DrawRoiContext::new());

    let canvas_ctx = draw_roi_ctx.canvas_ctx();
    let mut draw_ctx = draw_roi_ctx.draw_ctx();

    use_effect(move || {
        let Some(ctx) = canvas_ctx() else {
            return;
        };
        draw_ctx.read().redraw(&ctx);
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
        tracing::info!("on_wheel {:?}", e.data());
        let delta = e.delta().strip_units().y;
        draw_roi_ctx.draw_ctx().write().canvas_wheel(delta);
    };

    let on_mouse_move = move |e: MouseEvent| {
        e.prevent_default();
        draw_roi_ctx.write().canvas_move(&e);
    };

    let on_mouse_leave = move |e: MouseEvent| {
        e.prevent_default();
        tracing::info!("on_mouse_leave {:?}", e.data());
        draw_roi_ctx.write().canvas_leave(&e);
    };

    let on_mouse_down = move |e: MouseEvent| {
        e.prevent_default();
        draw_roi_ctx.write().canvas_mouse_down(&e);
    };

    let on_click = move |e: MouseEvent| {
        e.prevent_default();
        draw_roi_ctx.write().canvas_click(&e);
    };
    let on_double_click = move |e: MouseEvent| {
        e.prevent_default();
        draw_roi_ctx.write().canvas_double_click(&e);
    };

    let on_touch_start = move |e: TouchEvent| {
        e.prevent_default();
        tracing::info!("on_touch_start {:?}", e.data());
    };

    let on_touch_move = move |e: TouchEvent| {
        e.prevent_default();
        tracing::info!("on_touch_move {:?}", e.data());
    };

    let on_touch_end = move |e: TouchEvent| {
        e.prevent_default();
        tracing::info!("on_touch_end {:?}", e.data());
    };

    let draw_ctx_clone = draw_ctx();
    let rois_content = draw_ctx_clone
        .drawed_rois
        .iter()
        .enumerate()
        .map(|(idx, (name, roi))| {
            let roi_text = roi_to_string(roi);
            rsx! {
                div { key: "{idx}", class: "flex items-center gap-2",
                    div { "{name}" }
                    div { class: "font-mono text-sm", {roi_text} }
                }
            }
        });
    let draw_ctx_clone = draw_ctx();
    let all_rois = rois_to_string(&draw_ctx_clone.drawed_rois);

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

            div { class: "grid grid-cols-1 md:grid-cols-[1fr_auto] gap-2 items-start",

                AspectRatio { ratio: 16. / 9.,
                    canvas {
                        class: "w-full h-full border",
                        width: 1920,
                        height: 1080,
                        ontouchstart: on_touch_start,
                        ontouchmove: on_touch_move,
                        ontouchend: on_touch_end,
                        onclick: on_click,
                        ondoubleclick: on_double_click,
                        onwheel: on_wheel,
                        onmousedown: on_mouse_down,
                        onmousemove: on_mouse_move,
                        onmouseleave: on_mouse_leave,

                        onresize: move |e| {
                            tracing::info!("canvas.onresize {:?}", e.data());
                            draw_roi_ctx.draw_ctx().write().canvas_resize(&e);
                        },
                        onmounted: move |e| {
                            tracing::info!("canvas.onmounted");
                            draw_roi_ctx.write().canvas_mounted(&e);
                        },
                    }
                }

                div { class: "grid grid-cols-1 gap-2",
                    Toolbar {
                        ToolbarGroup {
                            ToolbarButton { index: 0usize,
                                Icon { icon: fa_solid_icons::FaHand }
                            }
                            ToolbarButton { index: 1usize,
                                Icon { icon: fa_solid_icons::FaSquarePlus }
                            }
                            ToolbarButton { index: 2usize,
                                Icon { icon: fa_solid_icons::FaPencil }
                            }
                            ToolbarButton { index: 3usize,
                                Icon { icon: fa_solid_icons::FaSquareMinus }
                            }
                        }
                    }

                    ScrollArea {
                        class: "border border-(--primary-color-6) grid grid-cols-1 gap-2 p-2",
                        direction: ScrollDirection::Vertical,

                        div { class: "flex",
                            div { class: "flex-1", "ROI 1" }
                            div {
                                Button { variant: ButtonVariant::Secondary,
                                    Icon { icon: fa_solid_icons::FaPencil }
                                }
                            }
                            div {
                                Button { variant: ButtonVariant::Secondary,
                                    Icon { icon: fa_solid_icons::FaSquareMinus }
                                }
                            }
                        
                        }

                        div { class: "flex",
                            div { class: "flex-1", "ROI 2" }
                            div {
                                Button { variant: ButtonVariant::Secondary,
                                    Icon { icon: fa_solid_icons::FaPencil }
                                }
                            }
                            div {
                                Button { variant: ButtonVariant::Secondary,
                                    Icon { icon: fa_solid_icons::FaSquareMinus }
                                }
                            }
                        
                        }
                        div { class: "flex",
                            div { class: "flex-1", "ROI 3" }
                            div {
                                Button { variant: ButtonVariant::Secondary,
                                    Icon { icon: fa_solid_icons::FaPencil }
                                }
                            }
                            div {
                                Button { variant: ButtonVariant::Secondary,
                                    Icon { icon: fa_solid_icons::FaSquareMinus }
                                }
                            }
                        
                        }
                    }
                
                }
            
            }

            // 已繪製 ROI 的內容顯示區域
            div { class: "border rounded p-3 max-h-40 overflow-y-auto",
                if draw_ctx().drawed_rois.is_empty() {
                    p { class: "text-sm", "尚未繪製任何 ROI" }
                } else {
                    {rois_content}
                }
            }

            // 所有 ROI 串成一行的顯示區域，附帶複製按鈕
            if !draw_ctx().drawed_rois.is_empty() {
                div { class: "border rounded p-3 space-y-2 mt-2",
                    h3 { class: "text-sm font-semibold", "完整 ROI 資料" }
                    div { class: "flex gap-2 items-start",
                        div { class: "flex-1 border rounded p-2 overflow-x-auto",
                            code {
                                id: "all_rois",
                                class: "text-xs font-mono break-all whitespace-pre-wrap",
                                "{all_rois}"
                            }
                        }
                        Button { "onclick": "navigator.clipboard.writeText(document.getElementById('all_rois').textContent);",
                            "複製"
                        }
                    }
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

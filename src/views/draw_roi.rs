use std::time::Duration;

use crate::components::{
    aspect_ratio::AspectRatio,
    button::{Button, ButtonVariant},
    input::Input,
    scroll_area::ScrollArea,
    toolbar::{Toolbar, ToolbarButton, ToolbarGroup},
};
use async_std::task::sleep;
use base64::prelude::*;
use dioxus::{
    html::{geometry::ElementPoint, input_data::MouseButton},
    logger::tracing,
    prelude::*,
    web::WebEventExt,
};
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::scroll_area::ScrollDirection;
use euclid::{point2, size2, Point2D, Size2D};
use indexmap::IndexMap;
use web_sys::{
    wasm_bindgen::JsCast, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum ToolMode {
    View,
    #[default]
    Draw,
    Edit,
    Delete,
}

#[derive(Default, Debug, Clone, Store)]
struct ToolStatus {
    mode: ToolMode,
    mouse_down_pos: Option<(f64, f64)>,
    is_dragging: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct EditStatus {
    target: String,
    near: Option<usize>,
    drag: Option<usize>,
}

impl EditStatus {
    fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum HightlightStatus {
    #[default]
    None,
    View(String),
    Edit(EditStatus),
}

struct Pixel;

#[derive(Debug, Clone)]
struct DrawProxy;

impl DrawProxy {
    fn new() -> Self {
        Self {}
    }

    async fn init(&self) -> bool {
        let ret =
            document::eval(r#"return window.roiHandler.init("draw_roi_image", "draw_roi_canvas")"#)
                .await;
        tracing::info!("proxy init {:?}", ret);
        if let Ok(ret) = ret {
            if let Some(ret) = ret.as_bool() {
                return ret;
            }
        }
        false
    }

    async fn set_scale(&self, scale: f64) {
        let prog = format!("window.roiHandler.setScale({scale})");
        let _ = document::eval(&prog).await;
    }

    async fn set_offset(&self, x: f64, y: f64) {
        let prog = format!("window.roiHandler.setOffset({x}, {y})");
        let _ = document::eval(&prog).await;
        // tracing::info!("proxy set_offset");
    }

    async fn set_mouse(&self, x: f64, y: f64) {
        let prog = format!("window.roiHandler.setMouse({x}, {y})");
        let _ = document::eval(&prog).await;
        // tracing::info!("proxy set_mouse");
    }

    async fn clear_mouse(&self) {
        let _ = document::eval("window.roiHandler.clearMouse()").await;
    }

    async fn set_current_points(&self, current_points: Vec<Point2D<i32, Pixel>>) {
        let mut vec_str = "[".to_string();
        vec_str.push_str(
            current_points
                .iter()
                .map(|v| format!("[{},{}]", v.x, v.y))
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        vec_str.push(']');
        let prog = format!(
            r#"let tmp = {vec_str};
               window.roiHandler.setCurrentPoints(tmp);"#
        );
        let _ = document::eval(&prog).await;
    }

    async fn redraw(&self) {
        let _ = document::eval("window.roiHandler.redraw()").await;
    }
}

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
    highlight: HightlightStatus,
    next_roi_id: usize,
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
            highlight: HightlightStatus::default(),
            next_roi_id: 0,
        }
    }

    fn to_canvas_pos(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.canvas_width / self.display_width / self.scale,
            y * self.canvas_height / self.display_height / self.scale,
        )
    }

    fn update_mouse_xy(&mut self, x: f64, y: f64) {
        self.mouse_display_xy = Some((x, y));
        self.mouse_canvas_xy = Some(self.to_canvas_pos(x, y));
    }

    fn mouse_leave(&mut self) {
        self.mouse_display_xy = None;
        self.mouse_canvas_xy = None;
    }

    fn add_offset_xy(&mut self, x: f64, y: f64) {
        self.offset_x += x;
        self.offset_y += y;
    }

    fn add_point(&mut self) {
        if let Some(canvas_xy) = self.mouse_canvas_xy {
            let offset = size2(-self.offset_x, -self.offset_y);
            let point = point2(canvas_xy.0, canvas_xy.1).add_size(&offset).to_i32();
            self.current_points.push(point);
            tracing::info!("新增點：{:?}", point);
        }
    }

    fn pop_last_point(&mut self, repeat: i32) {
        for _ in 0..repeat {
            self.current_points.pop();
        }
    }

    fn close_current_roi(&mut self) {
        if self.current_points.len() > 2 {
            // 閉合目前多邊形並新增至已完成清單
            let completed_roi = self.current_points.clone();
            self.insert_new_roi(completed_roi);
            self.current_points.clear();
            tracing::info!("已閉合多邊形，共有 ROI 數：{}", self.drawed_rois.len());
        } else {
            tracing::info!("無法閉合多邊形 - 需要至少 3 個點");
        }
    }

    fn get_drawed_rois(&self) -> IndexMap<String, Vec<Point2D<i32, Pixel>>> {
        self.drawed_rois.clone()
    }

    fn get_drawed_rois_key(&self) -> Vec<String> {
        self.drawed_rois.keys().cloned().collect::<Vec<_>>()
    }

    fn remove_drawed_roi(&mut self, name: &str) {
        self.drawed_rois.shift_remove(name);
    }

    fn is_drawed_roi_empty(&self) -> bool {
        self.drawed_rois.is_empty()
    }

    fn canvas_resize(&mut self, e: &ResizeEvent) {
        let s = e.get_content_box_size().unwrap();
        self.display_height = s.height;
        self.display_width = s.width;
    }

    fn redraw(&self, ctx: &CanvasRenderingContext2d) {
        // DrawContext::redraw0(self, ctx);
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

        // 繪製高亮的ROI（紅色，線寬更寬）
        match &draw_ctx.highlight {
            HightlightStatus::None => {}
            HightlightStatus::View(target) | HightlightStatus::Edit(EditStatus { target, .. }) => {
                if let Some(roi) = draw_ctx.drawed_rois.get(target) {
                    ctx.set_stroke_style_str("red");
                    ctx.set_fill_style_str("red");
                    ctx.set_line_width(4.0); // 比普通線條更粗
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

                    // 繪製高亮多邊形的頂點（紅色，較大）
                    for p in roi.iter() {
                        let canvas_xy = p.to_f64().add_size(&offset_xy);
                        ctx.begin_path();
                        ctx.arc(
                            canvas_xy.x,
                            canvas_xy.y,
                            6.0, // 比普通頂點更大
                            0.0,
                            std::f64::consts::PI * 2.0,
                        )
                        .unwrap();
                        ctx.fill();
                    }
                }
            }
        }

        // 繪製滑鼠十字準線
        ctx.set_stroke_style_str("black");
        ctx.set_fill_style_str("black");
        if let Some(xy) = &draw_ctx.mouse_canvas_xy {
            ctx.set_line_width(1.0);
            ctx.begin_path();
            ctx.move_to(xy.0 - 20., xy.1);
            ctx.line_to(xy.0 + 20., xy.1);
            ctx.move_to(xy.0, xy.1 - 20.);
            ctx.line_to(xy.0, xy.1 + 20.);
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

    fn highlight_clear(&mut self) {
        self.highlight = HightlightStatus::None;
    }

    fn highlight_view(&mut self, target: String) {
        if self.drawed_rois.contains_key(&target) {
            self.highlight = HightlightStatus::View(target);
        }
    }

    fn highlight_edit(&mut self, target: String) {
        if self.drawed_rois.contains_key(&target) {
            self.highlight = HightlightStatus::Edit(EditStatus::new(&target));
        }
    }

    fn insert_new_roi(&mut self, completed_roi: Vec<Point2D<i32, Pixel>>) {
        let mut roi_name = format!("ROI {}", self.next_roi_id); // 用計數器生成名稱
        while self.drawed_rois.contains_key(&roi_name) {
            self.next_roi_id += 1;
            roi_name = format!("ROI {}", self.next_roi_id);
        }
        self.drawed_rois.insert(roi_name, completed_roi);
        self.next_roi_id += 1; // 每次遞增
    }
}

#[derive(Debug, Clone, Store)]
struct DrawRoiContext {
    tool_ctx: ToolStatus,
    draw_proxy: DrawProxy,
    draw_ctx: DrawContext,
    canvas_ctx: Option<CanvasRenderingContext2d>,
    canvas_ref: Option<HtmlCanvasElement>,
}

impl DrawRoiContext {
    fn new() -> Self {
        Self {
            tool_ctx: Default::default(),
            draw_proxy: DrawProxy::new(),
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

    fn update_mouse_xy(&mut self, xy: &ElementPoint) {
        self.draw_ctx.update_mouse_xy(xy.x, xy.y);
    }

    fn add_offset_xy(&mut self, x: f64, y: f64) {
        self.draw_ctx.add_offset_xy(x, y);
    }

    fn canvas_move(&mut self, e: &MouseEvent) {
        let xy = e.element_coordinates();
        self.update_mouse_xy(&xy);

        if e.held_buttons().contains(MouseButton::Primary) {
            // 偵測是否發生了足夠的拖曳（超過 5 像素閾值）
            if let Some((down_x, down_y)) = &self.tool_ctx.mouse_down_pos {
                let dx = xy.x - down_x;
                let dy = xy.y - down_y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance > 5.0 {
                    self.tool_ctx.is_dragging = true;
                }
            }

            let we = e.as_web_event();
            let move_canvas_xy = self
                .draw_ctx
                .to_canvas_pos(we.movement_x() as f64, we.movement_y() as f64);
            self.add_offset_xy(move_canvas_xy.0, move_canvas_xy.1);
        }
    }

    fn canvas_leave(&mut self, _e: &MouseEvent) {
        self.draw_ctx.mouse_leave();
    }

    fn canvas_mouse_down(&mut self, e: &MouseEvent) {
        let xy = e.element_coordinates();
        let tool_ctx = &mut self.tool_ctx;
        tool_ctx.mouse_down_pos = Some((xy.x, xy.y));
        tool_ctx.is_dragging = false;
        tracing::info!("on_mouse_down at {:?}", tool_ctx.mouse_down_pos);
    }

    fn canvas_click(&mut self, _e: &MouseEvent) {
        let tool_ctx = &mut self.tool_ctx;

        // 只有在沒有拖曳時才認為是有效的點擊
        if tool_ctx.is_dragging {
            tracing::info!("忽略拖曳後的點擊");
            tool_ctx.mouse_down_pos = None;
            return;
        }
        tool_ctx.mouse_down_pos = None;
        if self.tool_ctx.mode == ToolMode::Draw {
            self.draw_ctx.add_point();
        }
    }

    fn canvas_double_click(&mut self, _e: &MouseEvent) {
        let tool_ctx = &mut self.tool_ctx;

        tool_ctx.mouse_down_pos = None;
        tool_ctx.is_dragging = false;

        if self.tool_ctx.mode == ToolMode::Draw {
            // 雙擊之前會有兩次單擊，要刪掉
            self.draw_ctx.pop_last_point(2);
            self.draw_ctx.close_current_roi();
        }
    }
}

#[store]
impl<Lens> Store<DrawRoiContext, Lens> {
    fn set_tool_mode(&mut self, mode: ToolMode) {
        match mode {
            ToolMode::View => {
                self.draw_ctx().write().current_points.clear();
            }
            ToolMode::Draw => {}
            ToolMode::Edit => {}
            ToolMode::Delete => {}
        }
        self.draw_ctx().write().highlight_clear();
        self.tool_ctx().mode().set(mode);
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

    use_future(move || async move {
        loop {
            let ret = draw_roi_ctx().draw_proxy.init().await;
            if ret {
                break;
            }
            sleep(Duration::from_secs(2)).await;
        }
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

    let on_image_load = move |_| async move {
        draw_ctx.write();
        let draw_proxy = draw_roi_ctx.draw_proxy().read().cloned();
        draw_proxy.redraw().await;
    };

    // 縮放：滑鼠滾輪 / 手機雙指捏合
    let on_wheel = move |e: Event<WheelData>| async move {
        e.prevent_default();
        tracing::info!("on_wheel {:?}", e.data());
        let delta = e.delta().strip_units().y;
        draw_roi_ctx.draw_ctx().write().canvas_wheel(delta);
        let draw_ctx = draw_roi_ctx.draw_ctx().read().cloned();
        let draw_proxy = draw_roi_ctx.draw_proxy().read().cloned();
        let scale = draw_ctx.scale;
        let (x, y) = (draw_ctx.offset_x, draw_ctx.offset_y);
        draw_proxy.set_offset(x, y).await;
        draw_proxy.set_scale(scale).await;
        match draw_ctx.mouse_canvas_xy {
            Some((x, y)) => {
                draw_proxy.set_mouse(x, y).await;
            }
            None => {
                draw_proxy.clear_mouse().await;
            }
        }
        draw_proxy.redraw().await;
    };

    let on_mouse_move = move |e: MouseEvent| async move {
        e.prevent_default();
        draw_roi_ctx.write().canvas_move(&e);

        let draw_ctx = draw_roi_ctx.draw_ctx().read().cloned();
        let draw_proxy = draw_roi_ctx.draw_proxy().read().cloned();
        match draw_ctx.mouse_canvas_xy {
            Some((x, y)) => {
                draw_proxy.set_mouse(x, y).await;
            }
            None => {
                draw_proxy.clear_mouse().await;
            }
        }
        draw_proxy
            .set_offset(draw_ctx.offset_x, draw_ctx.offset_y)
            .await;
        draw_proxy.redraw().await;
    };

    let on_mouse_leave = move |e: MouseEvent| async move {
        e.prevent_default();
        tracing::info!("on_mouse_leave {:?}", e.data());
        draw_roi_ctx.write().canvas_leave(&e);

        let draw_ctx = draw_roi_ctx.draw_ctx().read().cloned();
        let draw_proxy = draw_roi_ctx.draw_proxy().read().cloned();
        match draw_ctx.mouse_canvas_xy {
            Some((x, y)) => {
                draw_proxy.set_mouse(x, y).await;
            }
            None => {
                draw_proxy.clear_mouse().await;
            }
        }
        draw_proxy.redraw().await;
    };

    let on_mouse_down = move |e: MouseEvent| {
        e.prevent_default();
        draw_roi_ctx.write().canvas_mouse_down(&e);
    };

    let on_click = move |e: MouseEvent| async move {
        e.prevent_default();
        draw_roi_ctx.write().canvas_click(&e);

        let draw_ctx = draw_roi_ctx.draw_ctx().read().cloned();
        let draw_proxy = draw_roi_ctx.draw_proxy().read().cloned();
        draw_proxy.set_current_points(draw_ctx.current_points).await;
        draw_proxy.redraw().await;
    };

    let on_double_click = move |e: MouseEvent| async move {
        e.prevent_default();
        draw_roi_ctx.write().canvas_double_click(&e);

        let draw_ctx = draw_roi_ctx.draw_ctx().read().cloned();
        let draw_proxy = draw_roi_ctx.draw_proxy().read().cloned();
        draw_proxy.set_current_points(draw_ctx.current_points).await;
        draw_proxy.redraw().await;
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

    let drawed_rois = draw_ctx.read().get_drawed_rois();
    let rois_content = drawed_rois.iter().map(|(name, roi)| {
        let roi_text = roi_to_string(roi);
        rsx! {
            div { key: "{name}", class: "flex items-center gap-2",
                div { "{name}" }
                div { class: "font-mono text-sm", {roi_text} }
            }
        }
    });
    let drawed_rois = draw_ctx.read().get_drawed_rois();
    let all_rois = rois_to_string(&drawed_rois);

    let roi_keys = draw_ctx.read().get_drawed_rois_key();
    let rois_list = roi_keys.iter().map(|k| {
        let k = k.clone();
        let k2 = k.clone();
        let k3 = k.clone();
        rsx!{
            div { class: "flex", key: "ROI_{k}",
                div { class: "flex-1", "{k}" }
                div { class: if draw_roi_ctx.tool_ctx().mode() == ToolMode::View { "" } else { "hidden" },
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let k = k3.clone();
                            draw_roi_ctx.draw_ctx().write().highlight_view(k);
                        },
                        Icon { icon: fa_solid_icons::FaLightbulb }
                    }
                }

                div { class: if draw_roi_ctx.tool_ctx().mode() == ToolMode::Edit { "" } else { "hidden" },
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let k = k.clone();
                            draw_roi_ctx.draw_ctx().write().highlight_edit(k);
                        },
                        Icon { icon: fa_solid_icons::FaPencil }
                    }
                }
                div { class: if draw_roi_ctx.tool_ctx().mode() == ToolMode::Delete { "" } else { "hidden" },
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let k = k2.clone();
                            draw_roi_ctx.draw_ctx().write().remove_drawed_roi(&k);
                        },
                        Icon { icon: fa_solid_icons::FaSquareMinus }
                    }
                }
            }

        }});

    let canvas_api = asset!("/assets/canvas-api.js");

    rsx! {
        p { "🚧施工中🚧" }
        script { src: canvas_api }

        Button {
            onclick: |_| async {
                let _ = document::eval("window.roiHandler.helloworld();").await;
            },
            "TEST"
        }

        img {
            id: "draw_roi_image",
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
                        id: "draw_roi_canvas",
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

                        onresize: move |e| async move {
                            tracing::info!("canvas.onresize {:?}", e.data());
                            draw_roi_ctx.write().draw_ctx.canvas_resize(&e);
                        },
                        onmounted: move |e| async move {
                            tracing::info!("canvas.onmounted");
                            draw_roi_ctx.write().canvas_mounted(&e);
                        },
                    }
                }

                div { class: "grid grid-cols-1 gap-2",
                    Toolbar {
                        ToolbarGroup {
                            ToggleToolbarButton {
                                index: 0usize,
                                is_on: draw_roi_ctx.tool_ctx().mode() == ToolMode::View,
                                on_click: move || draw_roi_ctx.set_tool_mode(ToolMode::View),
                                Icon { icon: fa_solid_icons::FaHand }
                            }
                            ToggleToolbarButton {
                                index: 1usize,
                                is_on: draw_roi_ctx.tool_ctx().mode() == ToolMode::Draw,
                                on_click: move || draw_roi_ctx.set_tool_mode(ToolMode::Draw),

                                Icon { icon: fa_solid_icons::FaSquarePlus }
                            }
                            ToggleToolbarButton {
                                index: 2usize,
                                is_on: draw_roi_ctx.tool_ctx().mode() == ToolMode::Edit,
                                on_click: move || draw_roi_ctx.set_tool_mode(ToolMode::Edit),

                                Icon { icon: fa_solid_icons::FaPencil }
                            }
                            ToggleToolbarButton {
                                index: 3usize,
                                is_on: draw_roi_ctx.tool_ctx().mode() == ToolMode::Delete,
                                on_click: move || draw_roi_ctx.set_tool_mode(ToolMode::Delete),
                                Icon { icon: fa_solid_icons::FaSquareMinus }
                            }
                        }
                    }

                    ScrollArea {
                        class: "border border-(--primary-color-6) grid grid-cols-1 gap-2 p-2 max-h-64",
                        direction: ScrollDirection::Vertical,
                        if draw_ctx().is_drawed_roi_empty() {
                            p { class: "text-sm", "ROI列表顯示在此" }
                        } else {
                            {rois_list}
                        }
                    }
                }
            }

            // 已繪製 ROI 的內容顯示區域
            div { class: "border rounded p-3 max-h-40 overflow-y-auto",
                if draw_ctx().is_drawed_roi_empty() {
                    p { class: "text-sm", "尚未繪製任何 ROI" }
                } else {
                    {rois_content}
                }
            }

            // 所有 ROI 串成一行的顯示區域，附帶複製按鈕
            if !draw_ctx().is_drawed_roi_empty() {
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
                            Icon { icon: fa_solid_icons::FaClipboard }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ToggleToolbarButton(
    index: usize,
    is_on: bool,
    on_click: Callback<()>,
    children: Element,
) -> Element {
    rsx! {
        ToolbarButton {
            index,
            on_click,
            "data-state": if is_on { "on" } else { "off" },
            background: if is_on { "var(--light, var(--primary-color-5)) var(--dark, var(--primary-color-6))" } else { "" },
            color: if is_on { "var(--secondary-color-1)" } else { "" },
            {children}
        }
    }
}

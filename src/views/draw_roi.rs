use std::{default, time::Duration};

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
    html::{
        geometry::ElementPoint,
        input_data::{MouseButton, MouseButtonSet},
    },
    logger::tracing,
    prelude::*,
};
use dioxus_free_icons::{icons::fa_solid_icons, Icon};
use dioxus_primitives::scroll_area::ScrollDirection;
use euclid::{point2, size2, Point2D, UnknownUnit};
use indexmap::{IndexMap, IndexSet};
use time::{OffsetDateTime, PrimitiveDateTime, Time};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum ToolMode {
    View,
    #[default]
    Draw,
    Edit,
    Delete,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum GestureMode {
    #[default]
    None,
    Drag,
    Draw,
    Zoom,
}

#[derive(Default, Debug, Clone, Store)]
struct ToolStatus {
    mode: ToolMode,
    mouse_down_pos: Option<(f64, f64)>,
    last_mouse_pos: Option<(f64, f64)>,
    is_dragging: bool,

    last_pointer_pos: IndexMap<i32, (f64, f64)>,
    primary_buttons: IndexSet<i32>,
    last_click_time: Option<OffsetDateTime>,
    gesture: GestureMode,
}

impl ToolStatus {
    fn get_movement(&self, pointer_id: i32, coord: (f64, f64)) -> (f64, f64) {
        if let Some(last_pos) = self.last_pointer_pos.get(&pointer_id) {
            (coord.0 - last_pos.0, coord.1 - last_pos.1)
        } else {
            (0.0, 0.0)
        }
    }

    fn update_last_pos(&mut self, pointer_id: i32, coord: (f64, f64), btn: MouseButtonSet) {
        self.last_pointer_pos.insert(pointer_id, coord);
        if btn.contains(MouseButton::Primary) {
            self.primary_buttons.insert(pointer_id);
        } else {
            self.primary_buttons.shift_remove(&pointer_id);
        }
    }

    fn get_first_id(&self) -> Option<i32> {
        self.last_pointer_pos.first().map(|(k, _)| *k)
    }

    fn remove_pointer_id(&mut self, pointer_id: i32) {
        self.last_pointer_pos.shift_remove(&pointer_id);
        self.primary_buttons.shift_remove(&pointer_id);
    }
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
        let ret = document::eval(
            r#"
            window.roiHandler = Object.create(window.roiHandlerProto);
            return window.roiHandler.init("draw_roi_image", "draw_roi_canvas", "magnifier_canvas");"#,
        )
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

    async fn add_drawed_roi(&self, name: String, points: Vec<Point2D<i32, Pixel>>) {
        let mut vec_str = "[".to_string();
        vec_str.push_str(
            points
                .iter()
                .map(|v| format!("[{},{}]", v.x, v.y))
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        vec_str.push(']');
        let prog = format!(
            r#"let tmp = {vec_str};
               window.roiHandler.addDrawedRoi("{name}", tmp);"#
        );
        let _ = document::eval(&prog).await;
    }

    async fn replace_all_drawed_roi(
        &self,
        drawed_rois: IndexMap<String, Vec<Point2D<i32, Pixel>>>,
    ) {
        let mut prog = format!("window.roiHandler.clearDrawedRoi();");
        for (name, roi) in drawed_rois.iter() {
            let mut vec_str = "[".to_string();
            vec_str.push_str(
                roi.iter()
                    .map(|v| format!("[{},{}]", v.x, v.y))
                    .collect::<Vec<_>>()
                    .join(",")
                    .as_str(),
            );
            vec_str.push(']');
            prog.push_str(
                format!(r#"window.roiHandler.addDrawedRoi("{name}", {vec_str});"#).as_str(),
            );
        }
        let _ = document::eval(&prog).await;
    }

    async fn set_highlight(&self, name: String) {
        let prog = format!(r#"window.roiHandler.setHighlight("{name}")"#);
        let _ = document::eval(&prog).await;
    }

    async fn clear_highlight(&self) {
        let prog = format!(r#"window.roiHandler.clearHighlight()"#);
        let _ = document::eval(&prog).await;
    }

    async fn redraw(&self) {
        let _ = document::eval("window.roiHandler.redraw()").await;
    }

    async fn execute(&self, command: &str) {
        let _ = document::eval(command).await;
    }
}

struct DrawCommandBuilder {
    commands: String,
}

impl DrawCommandBuilder {
    fn new() -> Self {
        Self {
            commands: String::new(),
        }
    }

    fn set_scale(&mut self, scale: f64) -> &mut Self {
        let prog = format!("window.roiHandler.setScale({scale});");
        self.commands.push_str(&prog);
        self
    }

    fn set_offset(&mut self, x: f64, y: f64) -> &mut Self {
        let prog = format!("window.roiHandler.setOffset({x}, {y});");
        self.commands.push_str(&prog);
        self
    }

    fn set_mouse(&mut self, x: f64, y: f64) -> &mut Self {
        let prog = format!("window.roiHandler.setMouse({x}, {y});");
        self.commands.push_str(&prog);
        self
    }

    fn clear_mouse(&mut self) -> &mut Self {
        let prog = "window.roiHandler.clearMouse();".to_string();
        self.commands.push_str(&prog);
        self
    }

    fn set_current_points(&mut self, current_points: Vec<Point2D<i32, Pixel>>) -> &mut Self {
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
        self.commands.push_str(&prog);
        self
    }

    fn add_drawed_roi(&mut self, name: String, points: Vec<Point2D<i32, Pixel>>) -> &mut Self {
        let mut vec_str = "[".to_string();
        vec_str.push_str(
            points
                .iter()
                .map(|v| format!("[{},{}]", v.x, v.y))
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        vec_str.push(']');
        let prog = format!(
            r#"let tmp = {vec_str};
               window.roiHandler.addDrawedRoi("{name}", tmp);"#
        );
        self.commands.push_str(&prog);
        self
    }

    fn replace_all_drawed_roi(
        &mut self,
        drawed_rois: IndexMap<String, Vec<Point2D<i32, Pixel>>>,
    ) -> &mut Self {
        let mut prog = format!("window.roiHandler.clearDrawedRoi();");
        for (name, roi) in drawed_rois.iter() {
            let mut vec_str = "[".to_string();
            vec_str.push_str(
                roi.iter()
                    .map(|v| format!("[{},{}]", v.x, v.y))
                    .collect::<Vec<_>>()
                    .join(",")
                    .as_str(),
            );
            vec_str.push(']');
            prog.push_str(
                format!(r#"window.roiHandler.addDrawedRoi("{name}", {vec_str});"#).as_str(),
            );
        }
        self.commands.push_str(&prog);
        self
    }

    fn set_highlight(&mut self, name: String) -> &mut Self {
        let prog = format!(r#"window.roiHandler.setHighlight("{name}");"#);
        self.commands.push_str(&prog);
        self
    }

    fn clear_highlight(&mut self) -> &mut Self {
        let prog = format!(r#"window.roiHandler.clearHighlight();"#);
        self.commands.push_str(&prog);
        self
    }

    fn redraw(&mut self) -> &mut Self {
        let prog = "window.roiHandler.redraw();".to_string();
        self.commands.push_str(&prog);
        self
    }

    async fn execute(&self, draw_proxy: &DrawProxy) {
        draw_proxy.execute(&self.commands).await;
    }
}

#[derive(Debug, Clone, Copy)]
struct PinchBase {
    mouse_center: (f64, f64),
    image_center: (f64, f64),
    display_distance: f64,
    base_scale: f64,
}

#[derive(Debug, Clone)]
struct DrawContext {
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
    pinch_base: Option<PinchBase>,
}

impl DrawContext {
    fn new() -> Self {
        Self {
            canvas_height: 1920.,
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
            pinch_base: None,
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

    fn set_pinch_base(&mut self, p1: (f64, f64), p2: (f64, f64)) {
        let base_scale = self.scale;
        let mouse_center = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        let canvas_center = self.to_canvas_pos(mouse_center.0, mouse_center.1);
        let image_center = (
            canvas_center.0 - self.offset_x,
            canvas_center.1 - self.offset_y,
        );

        let pp1: Point2D<f64, Pixel> = point2(p1.0, p1.1);
        let pp2: Point2D<f64, Pixel> = point2(p2.0, p2.1);
        let display_distance = pp1.distance_to(pp2);

        let pinch_base = PinchBase {
            mouse_center,
            image_center,
            base_scale,
            display_distance,
        };
        self.pinch_base = Some(pinch_base)
    }

    fn perform_pinch_zoom(&mut self, p1: (f64, f64), p2: (f64, f64)) {
        let Some(pinch_base) = self.pinch_base else {
            return;
        };

        let pp1: Point2D<_, Pixel> = point2(p1.0, p1.1);
        let pp2: Point2D<_, Pixel> = point2(p2.0, p2.1);
        let new_distance = pp1.distance_to(pp2);

        let new_scale = new_distance * pinch_base.base_scale / pinch_base.display_distance;
        let new_scale = new_scale.clamp(0.1, 5.0);

        self.scale = new_scale;

        let new_canvas_xy = self.to_canvas_pos((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        let image_xy = pinch_base.image_center;
        let new_offset_xy = (new_canvas_xy.0 - image_xy.0, new_canvas_xy.1 - image_xy.1);
        self.mouse_canvas_xy = Some(new_canvas_xy);
        self.offset_x = new_offset_xy.0;
        self.offset_y = new_offset_xy.1;
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

fn use_draw_roi_context() -> DrawRoiContext {
    let tool_ctx = use_signal(|| ToolStatus::default());
    let draw_proxy = use_signal(|| DrawProxy::new());
    let draw_ctx = use_signal(|| DrawContext::new());
    return DrawRoiContext {
        tool_ctx,
        draw_proxy,
        draw_ctx,
    };
}

#[derive(Debug, Clone, Copy)]
struct DrawRoiContext {
    tool_ctx: Signal<ToolStatus>,
    draw_proxy: Signal<DrawProxy>,
    draw_ctx: Signal<DrawContext>,
}

impl DrawRoiContext {
    fn update_mouse_xy(&self, xy: &ElementPoint) {
        let mut draw_ctx = self.draw_ctx;
        draw_ctx.write().update_mouse_xy(xy.x, xy.y);
    }

    fn add_offset_xy(&self, x: f64, y: f64) {
        let mut draw_ctx = self.draw_ctx;
        draw_ctx.write().add_offset_xy(x, y);
    }

    fn canvas_move(&self, e: &MouseEvent) {
        let DrawRoiContext {
            mut tool_ctx,
            draw_ctx,
            ..
        } = *self;
        let xy = e.element_coordinates();
        let movement = if let Some(last_pos) = tool_ctx.read().last_mouse_pos {
            (xy.x - last_pos.0, xy.y - last_pos.1)
        } else {
            (0.0, 0.0)
        };

        self.update_mouse_xy(&xy);

        if e.held_buttons().contains(MouseButton::Primary) {
            // 偵測是否發生了足夠的拖曳（超過 5 像素閾值）
            if let Some((down_x, down_y)) = tool_ctx().mouse_down_pos {
                let dx = xy.x - down_x;
                let dy = xy.y - down_y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance > 5.0 {
                    tool_ctx.write().is_dragging = true;
                }
            }

            let move_canvas_xy = draw_ctx.read().to_canvas_pos(movement.0, movement.1);
            self.add_offset_xy(move_canvas_xy.0, move_canvas_xy.1);
        }
        tool_ctx.write().last_mouse_pos = Some((xy.x, xy.y));
    }

    fn canvas_leave(&self, _e: &MouseEvent) {
        let DrawRoiContext {
            mut tool_ctx,
            mut draw_ctx,
            ..
        } = *self;
        draw_ctx.write().mouse_leave();
        tool_ctx.write().last_mouse_pos = None;
    }

    fn canvas_mouse_down(&self, e: &MouseEvent) {
        let xy = e.element_coordinates();
        let mut tool_ctx = self.tool_ctx;
        tool_ctx.with_mut(move |tool_ctx| {
            tool_ctx.mouse_down_pos = Some((xy.x, xy.y));
            tool_ctx.is_dragging = false;
        });
        tracing::info!("on_mouse_down at {:?}", tool_ctx.read().mouse_down_pos);
    }

    fn canvas_click(&self, _e: &MouseEvent) {
        let mut tool_ctx = self.tool_ctx;
        let mut tool_ctx = tool_ctx.write();

        // 只有在沒有拖曳時才認為是有效的點擊
        if tool_ctx.is_dragging {
            tracing::info!("忽略拖曳後的點擊");
            tool_ctx.mouse_down_pos = None;
            return;
        }
        tool_ctx.mouse_down_pos = None;

        if tool_ctx.mode == ToolMode::Draw {
            let mut draw_ctx = self.draw_ctx;
            draw_ctx.write().add_point();
        }
    }

    fn canvas_double_click(&self, _e: &MouseEvent) {
        let mut tool_ctx = self.tool_ctx;
        let mut tool_ctx = tool_ctx.write();

        tool_ctx.mouse_down_pos = None;
        tool_ctx.is_dragging = false;

        if tool_ctx.mode == ToolMode::Draw {
            // 雙擊之前會有兩次單擊，要刪掉
            let mut draw_ctx = self.draw_ctx;
            let mut draw_ctx = draw_ctx.write();
            draw_ctx.pop_last_point(2);
            draw_ctx.close_current_roi();
        }
    }

    async fn sync_pos_offset(&self, redraw: bool) {
        let draw_ctx = self.draw_ctx;
        let draw_ctx = draw_ctx();
        let draw_proxy = self.draw_proxy;
        let draw_proxy = draw_proxy();

        let mut builder = DrawCommandBuilder::new();
        match draw_ctx.mouse_canvas_xy {
            Some((x, y)) => {
                builder.set_mouse(x, y);
            }
            None => {
                builder.clear_mouse();
            }
        }
        builder.set_scale(draw_ctx.scale);
        builder.set_offset(draw_ctx.offset_x, draw_ctx.offset_y);
        if redraw {
            builder.redraw();
        }
        builder.execute(&draw_proxy).await;
    }

    async fn sync_all_roi(&self, redraw: bool) {
        let DrawRoiContext {
            draw_proxy,
            draw_ctx,
            ..
        } = *self;
        let draw_ctx = draw_ctx();
        let draw_proxy = draw_proxy();

        let mut builder = DrawCommandBuilder::new();
        builder.set_current_points(draw_ctx.current_points);
        builder.replace_all_drawed_roi(draw_ctx.drawed_rois);
        if redraw {
            builder.redraw();
        }
        builder.execute(&draw_proxy).await;
    }

    async fn canvas_pointer_down(&self, e: &PointerEvent) {
        let DrawRoiContext {
            mut tool_ctx,
            mut draw_ctx,
            ..
        } = *self;
        let xy = e.element_coordinates();

        {
            let mut tool_ctx = tool_ctx.write();
            tool_ctx.update_last_pos(e.pointer_id(), xy.to_tuple(), e.held_buttons());
            let clicked_ids = tool_ctx.primary_buttons.clone();
            match clicked_ids.len() {
                0 => {
                    tool_ctx.gesture = GestureMode::None;
                }
                1 => {
                    if tool_ctx.mode == ToolMode::Draw {
                        tool_ctx.gesture = GestureMode::Draw;
                    } else {
                        tool_ctx.gesture = GestureMode::Drag;
                    }
                }
                2 => {
                    tool_ctx.gesture = GestureMode::Zoom;
                    let p1 = tool_ctx
                        .last_pointer_pos
                        .get(&clicked_ids[0])
                        .cloned()
                        .unwrap();
                    let p2 = tool_ctx
                        .last_pointer_pos
                        .get(&clicked_ids[1])
                        .cloned()
                        .unwrap();
                    draw_ctx.write().set_pinch_base(p1, p2);
                }
                _ => {}
            }
        }

        self.sync_pos_offset(true).await;
    }

    async fn canvas_pointer_move(&self, e: &PointerEvent) {
        let mut tool_ctx = self.tool_ctx;
        let tool_ctx_copy = tool_ctx();
        let current_mode = tool_ctx_copy.mode;
        let current_gesture = tool_ctx_copy.gesture;
        let xy = e.element_coordinates();
        let movement = tool_ctx.read().get_movement(e.pointer_id(), xy.to_tuple());
        tool_ctx
            .write()
            .update_last_pos(e.pointer_id(), xy.to_tuple(), e.held_buttons());

        let first_id = tool_ctx.read().get_first_id().unwrap();
        if first_id == e.pointer_id() {
            self.update_mouse_xy(&xy);
        }

        match current_gesture {
            GestureMode::Drag => {
                if first_id == e.pointer_id() {
                    let draw_ctx = self.draw_ctx;
                    let move_canvas_xy = draw_ctx.read().to_canvas_pos(movement.0, movement.1);
                    self.add_offset_xy(move_canvas_xy.0, move_canvas_xy.1);
                }
            }
            GestureMode::Zoom => {
                let tool_ctx_copy = tool_ctx();
                let clicked_ids = tool_ctx_copy.primary_buttons.clone();
                if clicked_ids.len() >= 2 {
                    let p1 = tool_ctx_copy
                        .last_pointer_pos
                        .get(&clicked_ids[0])
                        .cloned()
                        .unwrap();
                    let p2 = tool_ctx_copy
                        .last_pointer_pos
                        .get(&clicked_ids[1])
                        .cloned()
                        .unwrap();
                    let mut draw_ctx = self.draw_ctx;
                    draw_ctx.write().perform_pinch_zoom(p1, p2);
                }
            }
            _ => {}
        }

        tool_ctx.write().last_mouse_pos = Some((xy.x, xy.y));

        self.sync_pos_offset(true).await;
    }

    async fn canvas_pointer_up(&self, e: &PointerEvent) {
        let xy = e.element_coordinates();
        let mut tool_ctx = self.tool_ctx;
        {
            let mut tool_ctx = tool_ctx.write();
            tool_ctx.update_last_pos(e.pointer_id(), xy.to_tuple(), e.held_buttons());

            if tool_ctx.gesture == GestureMode::Draw {
                let first_id = tool_ctx.get_first_id().unwrap();
                if first_id == e.pointer_id() {
                    // 新增一個點
                    let mut draw_ctx = self.draw_ctx;
                    let mut draw_ctx = draw_ctx.write();
                    draw_ctx.add_point();

                    let now = OffsetDateTime::now_utc();
                    if let Some(last_click_time) = tool_ctx.last_click_time {
                        if now - last_click_time < time::Duration::milliseconds(500) {
                            // 觸發雙擊
                            draw_ctx.pop_last_point(2);
                            draw_ctx.close_current_roi();
                        }
                    }

                    tool_ctx.last_click_time = Some(OffsetDateTime::now_utc());
                }
            }

            match tool_ctx.primary_buttons.len() {
                0 => {
                    tool_ctx.gesture = GestureMode::None;
                }
                1 => {
                    if tool_ctx.mode == ToolMode::Draw {
                        // 一律回到Drag才不會誤畫
                        tool_ctx.gesture = GestureMode::Drag;
                    } else {
                        tool_ctx.gesture = GestureMode::Drag;
                    }
                }
                _ => {
                    tool_ctx.gesture = GestureMode::Zoom;
                }
            }
        }

        self.sync_all_roi(true).await;
    }

    async fn canvas_pointer_cancel(&self, e: &PointerEvent) {
        let mut tool_ctx = self.tool_ctx;
        let mut draw_ctx = self.draw_ctx;
        {
            let mut tool_ctx = tool_ctx.write();
            let mut draw_ctx = draw_ctx.write();
            tool_ctx.remove_pointer_id(e.pointer_id());
            draw_ctx.mouse_leave();
            tool_ctx.last_mouse_pos = None;
        }

        self.sync_pos_offset(true).await;
    }

    async fn canvas_pointer_leave(&self, e: &PointerEvent) {
        let mut tool_ctx = self.tool_ctx;
        let mut draw_ctx = self.draw_ctx;
        {
            let mut tool_ctx = tool_ctx.write();
            let mut draw_ctx = draw_ctx.write();
            tool_ctx.remove_pointer_id(e.pointer_id());
            draw_ctx.mouse_leave();
            tool_ctx.last_mouse_pos = None;
        }

        self.sync_pos_offset(true).await;
    }

    async fn set_tool_mode(&self, mode: ToolMode) {
        let DrawRoiContext {
            mut tool_ctx,
            mut draw_ctx,
            draw_proxy,
        } = *self;
        match mode {
            ToolMode::View => {
                // self.draw_ctx().write().current_points.clear();
            }
            ToolMode::Draw => {}
            ToolMode::Edit => {}
            ToolMode::Delete => {}
        }
        draw_ctx.write().highlight_clear();
        tool_ctx.write().mode = mode;

        let draw_ctx = draw_ctx();
        let draw_proxy = draw_proxy();

        DrawCommandBuilder::new()
            .set_current_points(draw_ctx.current_points)
            .clear_highlight()
            .redraw()
            .execute(&draw_proxy)
            .await;
    }

    async fn remove_drawed_roi(&self, name: &str) {
        let mut draw_ctx = self.draw_ctx;
        draw_ctx.write().remove_drawed_roi(name);

        self.sync_all_roi(true).await;
    }

    async fn highlight_view(&self, name: &str) {
        let mut draw_ctx = self.draw_ctx;
        draw_ctx.write().highlight_view(name.to_string());

        let draw_proxy = self.draw_proxy;
        let draw_proxy = draw_proxy();
        DrawCommandBuilder::new()
            .set_highlight(name.to_string())
            .redraw()
            .execute(&draw_proxy)
            .await;
    }

    async fn highlight_edit(&self, name: &str) {
        let mut draw_ctx = self.draw_ctx;
        draw_ctx.write().highlight_edit(name.to_string());

        let draw_proxy = self.draw_proxy;
        let draw_proxy = draw_proxy();
        DrawCommandBuilder::new()
            .set_highlight(name.to_string())
            .redraw()
            .execute(&draw_proxy)
            .await;
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

    let mut draw_roi_ctx = use_draw_roi_context();
    let mut draw_ctx = draw_roi_ctx.draw_ctx;

    // use_future(move || async move {
    //     loop {
    //         let ret = draw_roi_ctx().draw_proxy.init().await;
    //         if ret {
    //             break;
    //         }
    //         sleep(Duration::from_secs(2)).await;
    //     }
    // });
    let mut canvas_width = use_signal(|| 1920);
    let mut canvas_height = use_signal(|| 1080);

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
        let draw_proxy = draw_roi_ctx.draw_proxy;
        draw_proxy().redraw().await;
    };

    // 縮放：滑鼠滾輪 / 手機雙指捏合
    let on_wheel = move |e: Event<WheelData>| async move {
        e.prevent_default();
        tracing::info!("on_wheel {:?}", e.data());
        let delta = e.delta().strip_units().y;
        let DrawRoiContext {
            tool_ctx,
            draw_proxy,
            mut draw_ctx,
        } = draw_roi_ctx;
        draw_ctx.write().canvas_wheel(delta);

        let draw_ctx = draw_ctx();
        let scale = draw_ctx.scale;
        let (x, y) = (draw_ctx.offset_x, draw_ctx.offset_y);
        let mut builder = DrawCommandBuilder::new();
        builder.set_offset(x, y);
        builder.set_scale(scale);
        match draw_ctx.mouse_canvas_xy {
            Some((x, y)) => {
                builder.set_mouse(x, y);
            }
            None => {
                builder.clear_mouse();
            }
        }
        builder.redraw();
        builder.execute(&draw_proxy()).await;
    };

    let on_mouse_move = move |e: MouseEvent| async move {
        e.prevent_default();
        let DrawRoiContext {
            tool_ctx,
            draw_proxy,
            draw_ctx,
        } = draw_roi_ctx;
        draw_roi_ctx.canvas_move(&e);

        let draw_ctx = draw_ctx();
        let draw_proxy = draw_proxy();
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
        let DrawRoiContext {
            tool_ctx,
            draw_proxy,
            draw_ctx,
        } = draw_roi_ctx;
        draw_roi_ctx.canvas_leave(&e);

        let draw_ctx = draw_ctx();
        let draw_proxy = draw_proxy();
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
        draw_roi_ctx.canvas_mouse_down(&e);
    };

    let on_click = move |e: MouseEvent| async move {
        e.prevent_default();
        draw_roi_ctx.canvas_click(&e);

        let DrawRoiContext {
            tool_ctx,
            draw_proxy,
            draw_ctx,
        } = draw_roi_ctx;
        let draw_ctx = draw_ctx();
        let draw_proxy = draw_proxy();
        draw_proxy.set_current_points(draw_ctx.current_points).await;
        draw_proxy.redraw().await;
    };

    let on_double_click = move |e: MouseEvent| async move {
        e.prevent_default();
        draw_roi_ctx.canvas_double_click(&e);

        let DrawRoiContext {
            tool_ctx,
            draw_proxy,
            draw_ctx,
        } = draw_roi_ctx;
        let draw_ctx = draw_ctx();
        let draw_proxy = draw_proxy();
        draw_proxy.set_current_points(draw_ctx.current_points).await;
        draw_proxy
            .replace_all_drawed_roi(draw_ctx.drawed_rois)
            .await;
        draw_proxy.redraw().await;
    };

    let on_pointer_down = move |e: PointerEvent| async move {
        e.prevent_default();
        tracing::info!("on_pointer_down {:?}", e);
        draw_roi_ctx.canvas_pointer_down(&e).await;
    };

    let on_pointer_move = move |e: PointerEvent| async move {
        e.prevent_default();
        tracing::info!("on_pointer_move {:?}", e);
        draw_roi_ctx.canvas_pointer_move(&e).await;
    };

    let on_pointer_up = move |e: PointerEvent| async move {
        e.prevent_default();
        tracing::info!("on_pointer_up {:?}", e);
        draw_roi_ctx.canvas_pointer_up(&e).await;
    };

    let on_pointer_cancel = move |e: PointerEvent| async move {
        e.prevent_default();
        tracing::info!("on_pointer_cancel {:?}", e);
        draw_roi_ctx.canvas_pointer_cancel(&e).await;
    };

    let on_pointer_leave = move |e: PointerEvent| async move {
        e.prevent_default();
        tracing::info!("on_pointer_leave {:?}", e);
        draw_roi_ctx.canvas_pointer_leave(&e).await;
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
        let k1 = k.clone();
        let k2 = k.clone();
        rsx!{
            div { class: "flex", key: "ROI_{k}",
                div { class: "flex-1", "{k}" }
                div { class: if draw_roi_ctx.tool_ctx.read().mode == ToolMode::View { "" } else { "hidden" },
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let k = k.clone();
                            async move {
                                draw_roi_ctx.highlight_view(&k).await;
                            }
                        },
                        Icon { icon: fa_solid_icons::FaLightbulb }
                    }
                }

                div { class: if draw_roi_ctx.tool_ctx.read().mode == ToolMode::Edit { "" } else { "hidden" },
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let k = k1.clone();
                            async move {
                                draw_roi_ctx.highlight_edit(&k).await;
                            }
                        },
                        Icon { icon: fa_solid_icons::FaPencil }
                    }
                }
                div { class: if draw_roi_ctx.tool_ctx.read().mode == ToolMode::Delete { "" } else { "hidden" },
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let k = k2.clone();
                            async move {
                                draw_roi_ctx.remove_drawed_roi(&k).await;
                            }
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

        img {
            id: "draw_roi_image",
            class: "hidden",
            src: selected_file(),
            onload: on_image_load,
        }

        div { class: "grid grid-cols-1 gap-2",

            Input { r#type: "file", accept: "image/*", oninput: on_file_input }

            div {
                canvas { id: "magnifier_canvas", width: 200, height: 200 }
            }

            div { class: "grid grid-cols-1 md:grid-cols-[1fr_auto] gap-2 items-start",

                canvas {
                    id: "draw_roi_canvas",
                    class: "w-full h-full border touch-none aspect-square md:aspect-video",
                    width: canvas_width(),
                    height: canvas_height(),
                    onwheel: on_wheel,
                    onpointerdown: on_pointer_down,
                    onpointermove: on_pointer_move,
                    onpointerup: on_pointer_up,
                    onpointercancel: on_pointer_cancel,
                    onpointerleave: on_pointer_leave,

                    onresize: move |e| async move {
                        tracing::info!("canvas.onresize {:?}", e.data());
                        let s = e.get_content_box_size().unwrap();
                        let display_width = s.width;
                        let display_height = s.height;

                        // 計算寬高比
                        let aspect_ratio = display_width / display_height;

                        // 根據寬高比決定 canvas 內部尺寸
                        let (new_canvas_width, new_canvas_height) = if aspect_ratio > 1.5 {
                            // 寬屏（電腦：16:9）
                            (1920, 1080)
                        } else {
                            // 接近正方形或竪屏（手機）
                            (1920, 1920)
                        };

                        canvas_width.set(new_canvas_width);
                        canvas_height.set(new_canvas_height);

                        // 更新 DrawContext
                        let mut draw_ctx_mut = draw_roi_ctx.draw_ctx;
                        draw_ctx_mut.write().canvas_resize(&e);
                        draw_ctx_mut.write().canvas_width = new_canvas_width as f64;
                        draw_ctx_mut.write().canvas_height = new_canvas_height as f64;
                    },
                    onmounted: move |_| async move {
                        loop {
                            let draw_proxy = draw_roi_ctx.draw_proxy;
                            let ret = draw_proxy().init().await;
                            if ret {
                                break;
                            }
                            sleep(Duration::from_secs(2)).await;
                        }
                    },
                }
                div { class: "grid grid-cols-1 gap-2",
                    Toolbar {
                        ToolbarGroup {
                            ToggleToolbarButton {
                                index: 0usize,
                                is_on: draw_roi_ctx.tool_ctx.read().mode == ToolMode::View,
                                on_click: move || async move {
                                    draw_roi_ctx.set_tool_mode(ToolMode::View).await;
                                },
                                Icon { icon: fa_solid_icons::FaHand }
                            }
                            ToggleToolbarButton {
                                index: 1usize,
                                is_on: draw_roi_ctx.tool_ctx.read().mode == ToolMode::Draw,
                                on_click: move || async move {
                                    draw_roi_ctx.set_tool_mode(ToolMode::Draw).await;
                                },

                                Icon { icon: fa_solid_icons::FaSquarePlus }
                            }
                            ToggleToolbarButton {
                                index: 2usize,
                                is_on: draw_roi_ctx.tool_ctx.read().mode == ToolMode::Edit,
                                on_click: move || async move {
                                    draw_roi_ctx.set_tool_mode(ToolMode::Edit).await;
                                },

                                Icon { icon: fa_solid_icons::FaPencil }
                            }
                            ToggleToolbarButton {
                                index: 3usize,
                                is_on: draw_roi_ctx.tool_ctx.read().mode == ToolMode::Delete,
                                on_click: move || async move {
                                    draw_roi_ctx.set_tool_mode(ToolMode::Delete).await;
                                },
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
        div { class: "h-96" }
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

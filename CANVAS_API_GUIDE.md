# Canvas API 快速參考

## 核心組件

### 1. Rust 端 (src/canvas_api.rs)
```rust
// 調用 Canvas 繪製
pub fn redraw_canvas(
    canvas_id: &str,
    config: &RedrawConfig,
    image_id: Option<&str>,
)

// 數據結構
pub struct RedrawConfig {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub drawed_rois: Vec<Vec<(i32, i32)>>,
    pub current_points: Vec<(i32, i32)>,
    pub mouse_xy: Option<(f64, f64)>,
    pub highlight: Option<HighlightConfig>,
}
```

### 2. JavaScript 端 (assets/canvas-api.js)
```javascript
// 全局函數 window.drawROICanvas(config)
window.CanvasDrawAPI.redraw(config)
```

### 3. DOM 元素需求
```html
<canvas id="roi-canvas" width="1920" height="1080"></canvas>
<img id="roi-image" class="hidden" src="..."/>
```

## 使用流程

### DrawContext → 繪製
```rust
// 在 DrawRoiPage 中
let draw_ctx = draw_roi_ctx.draw_ctx();

// 觸發繪製（effect 或事件處理器中）
draw_ctx.read().redraw(None);
```

### redraw() 方法
```rust
fn redraw(&self, _ctx: Option<()>) {
    // 將 DrawContext 的狀態轉換為 RedrawConfig
    let config = RedrawConfig { ... };
    
    // 調用 Canvas API（只在 web 上執行）
    redraw_canvas("roi-canvas", &config, self.image_src.as_deref());
}
```

## 條件編譯

### Web 目標
- 依賴啟用：`web-sys`, `wasm-bindgen`, `serde-wasm-bindgen`
- Canvas API：調用 JavaScript
- 編譯命令：`dx serve` 或 `cargo build --target wasm32-unknown-unknown --features web`

### Desktop 目標
- 依賴禁用：所有 web_sys 功能
- Canvas API：空實現（no-op）
- 編譯命令：`cargo build --features desktop`

## 常見操作

### 添加新的繪製功能

1. **在 JavaScript 中添加方法** (assets/canvas-api.js)
```javascript
window.CanvasDrawAPI.newFeature = function() {
    // 實現繪製邏輯
};
```

2. **在 Rust 中暴露函數** (src/canvas_api.rs)
```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "newFeature")]
    fn new_feature_js();
}

pub fn new_feature() {
    #[cfg(feature = "web")]
    new_feature_js();
}
```

3. **在 draw_roi.rs 中調用**
```rust
use crate::canvas_api::new_feature;

// 在需要的地方
new_feature();
```

## 故障排除

### 編譯錯誤：`as_web_event` 不存在
- ✅ 解決：確保 `as_web_event()` 調用被包裹在 `#[cfg(feature = "web")]` 中

### Canvas 不渲染
- ✅ 檢查：
  1. DOM 中是否存在 `id="roi-canvas"` 的 canvas 元素
  2. JavaScript 是否已載入（檢查 Dioxus.toml 中的 script 配置）
  3. `redraw()` 是否被正確調用

### Desktop 編譯失敗
- ✅ 檢查：確保未使用 `#[cfg(not(feature = "web"))]` 下的 web_sys 類型

## 性能考慮

1. **批量更新** - 使用單個 `redraw_canvas()` 呼叫而非多個
2. **圖像寶貴化** - Canvas 自動緩存背景圖像（JavaScript）
3. **路徑複用** - ROI 點數據轉換為向量，避免重複計算

## 未來計劃

- [ ] 實現 desktop 版本的 Canvas 渲染（使用 `wgpu` 或 `skia-rs`）
- [ ] 添加撤銷/重做功能
- [ ] 實現導出 ROI 為圖像
- [ ] 移動設備觸控優化

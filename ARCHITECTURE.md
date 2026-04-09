# Canvas API 架構圖

## 整體架構

```
┌─────────────────────────────────────────────────────────────────┐
│                    Dioxus 應用 (跨平台)                         │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │
                ┌─────────────┴──────────────┐
                │                            │
        ┌───────▼────────┐         ┌────────▼────────┐
        │  Web 目標      │         │  Desktop 目標   │
        │  (wasm32)      │         │  (x86_64)       │
        └───────┬────────┘         └────────┬────────┘
                │                           │
        ┌───────▼──────────────┐   ┌────────▼────────┐
        │  Canvas API Web      │   │ Canvas API      │
        │  實現 (src/...)      │   │ No-op (src/...) │
        │  ✓ wasm_bindgen      │   │ ✓ 空實現        │
        │  ✓ web_sys          │   │ ✗ 無 web_sys    │
        └───────┬──────────────┘   └────────┬────────┘
                │                           │
        ┌───────▼──────────────┐            │
        │  JavaScript API      │            │
        │  (assets/...)        │            │
        │  ✓ canvas 繪製邏輯    │            │
        └───────┬──────────────┘            │
                │                           │
        ┌───────▼──────────────┐   ┌────────▼────────┐
        │   Canvas DOM         │   │  （不可用）    │
        │   + Image DOM        │   │                │
        └──────────────────────┘   └─────────────────┘
```

## 數據流

```
┌──────────────────────────────┐
│   DrawContext (Rust State)   │
│  - drawed_rois              │
│  - current_points           │
│  - mouse_xy                 │
│  - highlight                │
│  - scale, offset, ...       │
└──────────────┬───────────────┘
               │
               │ redraw()
               ▼
┌──────────────────────────────┐
│    RedrawConfig             │
│  - Vec<Vec<(i32, i32)>>    │
│  - Vec<(i32, i32)>         │
│  - Option<HighlightConfig>  │
└──────────────┬───────────────┘
               │
      ┌────────┴─────────┐
      │                  │
  #[cfg(web)]      #[cfg(not(web))]
      │                  │
      ▼                  ▼
┌──────────────────────────────┐
│  serde_wasm_bindgen          │  No-op
│   - 序列化 to JsValue         │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│  window.drawROICanvas()      │
│  (wasm_bindgen extern "C")   │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│  CanvasDrawAPI.redraw()      │
│  (assets/canvas-api.js)      │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│  Canvas 2D API 繪製           │
│  - ctx.strokeStyle           │
│  - ctx.fillRect              │
│  - ctx.arc                   │
│  - ...                       │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│   DOM Canvas 顯示             │
│  <canvas id="roi-canvas">    │
└──────────────────────────────┘
```

## 文件結構

```
data-viewer/
├── src/
│   ├── main.rs                    ← mod canvas_api;
│   ├── canvas_api.rs              ← 新增：Canvas API 抽象層
│   └── views/
│       └── draw_roi.rs            ← 修改：使用 Canvas API
├── assets/
│   ├── canvas-api.js              ← 新增：JavaScript 實現
│   └── ...
├── Cargo.toml                      ← 修改：依賴配置
├── Dioxus.toml                     ← 修改：script 資源
├── REFACTORING.md                  ← 新增：重構說明
├── CANVAS_API_GUIDE.md             ← 新增：API 指南
└── COMPLETION_CHECKLIST.md         ← 新增：完成清單
```

## 模組依賴圖

```
main.rs
  ├── canvas_api.rs              [條件編譯]
  │   ├── #[cfg(feature="web")]
  │   │   ├── serde_wasm_bindgen
  │   │   ├── wasm_bindgen
  │   │   └── web_sys
  │   └── #[cfg(not(feature="web"))]
  │       └── （空實現）
  │
  └── views/
      └── draw_roi.rs
          ├── canvas_api          ✓
          ├── dioxus::web         [#[cfg(feature="web")]]
          └── euclid               ✓
```

## 編譯配置

```
.cargo/config.toml (隱式)
│
├── [target.wasm32-unknown-unknown]
│   └── features = ["web"]
│       ├── web-sys          ✓ 啟用
│       ├── wasm-bindgen     ✓ 啟用
│       └── serde-wasm-bindgen ✓ 啟用
│
└── [target.x86_64-unknown-linux-gnu]
    └── features = ["desktop"]
        ├── web-sys          ✗ 禁用
        ├── wasm-bindgen     ✗ 禁用
        └── serde-wasm-bindgen ✗ 禁用
```

## 代碼路徑示例

### Web 路徑（canvas 繪製）

```
MouseEvent on canvas
  ▼
canvas_move() 事件處理器
  ▼
update_mouse_xy() 更新 DrawContext
  ▼
use_effect 檢測變化
  ▼
draw_ctx.read().redraw(None)
  ▼
Rust 端：convert to RedrawConfig
  ▼
#[cfg(feature="web")] → serde_wasm_bindgen::to_value()
  ▼
wasm_bindgen extern "C" → window.drawROICanvas()
  ▼
JavaScript：CanvasDrawAPI.redraw()
  ▼
Canvas 2D API 繪製
  ▼
✓ 屏幕顯示
```

### Desktop 路徑（無 canvas 繪製）

```
MouseEvent on canvas
  ▼
canvas_move() 事件處理器
  ▼
update_mouse_xy() 更新 DrawContext
  ▼
use_effect 檢測變化
  ▼
draw_ctx.read().redraw(None)
  ▼
Rust 端：convert to RedrawConfig
  ▼
#[cfg(not(feature="web"))] → 空實現 (no-op)
  ▼
（無操作）
  ▼
✓ 編譯成功，但無視覺反饋
   （可在未來實現 native canvas）
```

## 條件編譯決策樹

```
是否編譯為 web?
│
├─ YES (--target wasm32-unknown-unknown --features web)
│  ├─ 啟用 web-sys ✓
│  ├─ 啟用 wasm_bindgen ✓
│  ├─ 啟用 serde-wasm-bindgen ✓
│  ├─ include canvas_api web_impl
│  ├─ include WebEventExt
│  ├─ include as_web_event() calls
│  └─ → 調用 JavaScript Canvas API
│
└─ NO (--features desktop)
   ├─ 禁用 web-sys
   ├─ 禁用 wasm_bindgen
   ├─ 禁用 serde-wasm-bindgen
   ├─ include canvas_api desktop_impl (no-op)
   ├─ exclude WebEventExt
   ├─ exclude as_web_event() calls
   └─ → 空實現，編譯但無功能
```

---

**優勢：同一套 Rust 代碼通過條件編譯支持多個平台！** 🎯

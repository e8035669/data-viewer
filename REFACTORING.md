# Canvas API 重構完成總結

## 📋 重構目標
使 Dioxus 項目能夠同時編譯成 **web** 和 **desktop** 目標，解決 `web_sys` 依賴只能在 web 上運行的問題。

## 🎯 實現方案

### 1. **新建 Canvas API 抽象層** ([src/canvas_api.rs](src/canvas_api.rs))
   - 創建**平台無關**的 Canvas 操作 API
   - 使用 Rust 的 `#[cfg(feature = "web")]` 條件編譯
   - **Web 目標**：透過 wasm-bindgen 調用 JavaScript API
   - **Desktop 目標**：提供空實現（no-op）

### 2. **創建 JavaScript Canvas API** ([assets/canvas-api.js](assets/canvas-api.js))
   - 完整的 Canvas 繪製邏輯，包括：
     - ROI 多邊形繪製（已完成的綠色、當前的紅色）
     - 頂點顯示
     - 滑鼠準線
     - 高亮功能
   - 自動從 DOM 查找 canvas 和 image 元素

### 3. **重構 DrawContext** ([src/views/draw_roi.rs](src/views/draw_roi.rs))
   - ❌ 移除 web_sys 依賴（`HtmlImageElement`, `CanvasRenderingContext2d`）
   - ✅ 改用字符串存儲圖像源（`image_src: Option<String>`）
   - ✅ 將原本的 canvas 上下文操作轉換為數據結構
   - ✅ 新增 `redraw()` 方法呼叫 Canvas API

### 4. **依賴管理更新** ([Cargo.toml](Cargo.toml))
```toml
# Web-only 依賴設為 optional
web-sys = { version = "0.3.94", ..., optional = true }
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.4", optional = true }

# 在 web 功能中啟用這些依賴
[features]
web = ["dioxus/web", "web-sys", "wasm-bindgen", "serde-wasm-bindgen"]
desktop = ["dioxus/desktop"]
```

### 5. **條件編譯修復**
   - ✅ `WebEventExt` 導入改為條件導入（只在 web 上可用）
   - ✅ `as_web_event()` 調用包裹在 `#[cfg(feature = "web")]` 中
   - ✅ Canvas 操作邏輯只在 web 上執行

### 6. **配置 Dioxus 項目** ([Dioxus.toml](Dioxus.toml))
```toml
[web.resource]
# 載入 JavaScript API
script = ["assets/canvas-api.js"]
```

## 🔧 關鍵技術

### Canvas API 數據流
```
Rust State (DrawContext)
    ↓
RedrawConfig (Serde序列化)
    ↓ [web target]
JavaScript (canvas-api.js)
    ↓
DOM Canvas 繪製
```

### 平台差異處理
```rust
// Web: 調用 JavaScript
#[cfg(feature = "web")]
mod web_impl { ... }

// Desktop: 空實現
#[cfg(not(feature = "web"))]
pub fn redraw_canvas(...) { /* no-op */ }
```

## ✅ 編譯結果

### Web 編譯
```bash
cargo build --target wasm32-unknown-unknown --no-default-features --features web
# ✓ 編譯成功
```

### Desktop 編譯 (Linux/macOS/Windows)
```bash
cargo build --target x86_64-unknown-linux-gnu --no-default-features --features desktop
# ✓ 編譯成功
```

## 📁 文件變更

### 新建
- `src/canvas_api.rs` - Canvas API 抽象層
- `assets/canvas-api.js` - JavaScript 實現

### 修改
- `src/main.rs` - 添加 canvas_api 模組
- `src/views/draw_roi.rs` - 移除 web_sys 依賴，集成新 API
- `Cargo.toml` - 依賴配置更新
- `Dioxus.toml` - JavaScript 資源配置

## 🚀 使用方式

### Web 開發
```bash
dx serve
# 使用預設的 web 功能編譯
```

### Desktop 開發
```bash
cargo run --target x86_64-unknown-linux-gnu --no-default-features --features desktop
```

## 💡 優勢

1. **單一代碼庫** - 相同的 Rust 代碼在 web 和 desktop 上運行
2. **無依賴衝突** - web_sys 只在 web 編譯時啟用
3. **易於維護** - Canvas 邏輯集中在 JavaScript 中，職責清晰
4. **高效執行** - Web 上直接使用原生 Canvas API，無額外開銷
5. **未來可擴展** - Desktop 版本可在未來實現本地 Canvas 渲染

## 📝 後續改進建議

1. **Desktop Canvas 實現** - 使用 `wgpu` 或 `skia-rs` 實現 desktop 版本的繪製
2. **動畫支持** - 添加 requestAnimationFrame 支持
3. **觸摸支持** - 完善移動設備的多點觸控
4. **性能優化** - 實現 dirty region 更新

# 🚀 Canvas API 集成指南

## 快速開始

### Web 開發
```bash
# 開發服務器
dx serve

# 編譯
dx build

# 訪問應用
# http://localhost:8080 (默認)

# 功能：✓ Canvas 繪制正常工作
```

### Desktop 開發
```bash
# Linux/macOS
cargo run --no-default-features --features desktop

# 編譯
cargo build --release --no-default-features --features desktop

# 功能：✓ 編譯成功，✗ Canvas 操作無效（待實現）
```

## 架構快速參考

```
DisplayContext (Rust State)
    ↓
RedrawConfig (序列化)
    ↓
┌─────────────────────────┐
│ #[cfg(feature="web")]   │
│ wasm-bindgen → JS       │ Web（調用 Canvas API）
├─────────────────────────┤
│ #[cfg(not(web))]        │
│ no-op                   │ Desktop（空實現）
└─────────────────────────┘
```

## 文件映射

| 文件 | 用途 | 最後修改 |
|------|------|---------|
| src/canvas_api.rs | Canvas API 抽象 | ✅ 新建 |
| assets/canvas-api.js | JavaScript 實現 | ✅ 新建 |
| src/views/draw_roi.rs | ROI 繪圖頁面 | ✅ 修改 |
| Cargo.toml | 依賴配置 | ✅ 修改 |
| Dioxus.toml | 資源配置 | ✅ 修改 |
| src/main.rs | 模組聲明 | ✅ 修改 |

## 常見任務

### 添加新的繪製功能

1. **在 JavaScript 中實現** (assets/canvas-api.js)
```javascript
window.CanvasDrawAPI.newFeature = function() {
    const canvas = document.getElementById("roi-canvas");
    if (!canvas) return;
    // 繪製邏輯
};
```

2. **在 Rust 中暴露** (src/canvas_api.rs)
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

3. **在 draw_roi.rs 中使用**
```rust
use crate::canvas_api::new_feature;
new_feature();
```

### 修改繪製數據結構

1. **更新 RedrawConfig** (src/canvas_api.rs)
```rust
pub struct RedrawConfig {
    // 添加新字段
    pub new_field: Vec<i32>,
}
```

2. **在 draw_roi.rs 中設置**
```rust
let config = RedrawConfig {
    new_field: some_data,
    // ...
};
```

3. **在 JavaScript 中使用**
```javascript
redraw(config) {
    const { new_field } = config;
    // 使用新字段
}
```

## 除錯技巧

### Web 除錯
```javascript
// 在 assets/canvas-api.js 中添加日誌
console.log('RedrawConfig:', config);
console.log('Canvas:', canvas);
```

### Rust 除錯
```rust
// 在 src/canvas_api.rs 中添加跟蹤
tracing::debug!("Redraw config: {:?}", config);
```

### 檢查條件編譯
```bash
# 驗證 web 特性
cargo build --target wasm32-unknown-unknown --features web --verbose 2>&1 | grep canvas_api

# 驗證 desktop 特性
cargo build --features desktop --verbose 2>&1 | grep canvas_api
```

## 故障排除

| 問題 | 原因 | 解決 |
|------|------|------|
| Canvas 不顯示 | JavaScript 未加載 | 檢查 Dioxus.toml script 配置 |
| 編譯錯誤：`as_web_event` | 未在 web 特性中 | 添加 `#[cfg(feature="web")]` |
| Canvas 為黑色 | 圖像未加載 | 檢查 image id 和 src |
| Desktop 編譯失敗 | web_sys 未禁用 | 使用 `--features desktop` |

## 效能考慮

### 優化建議
1. **批量更新** - 單次 redraw_canvas() 而非多次
2. **減少序列化** - 避免頻繁序列化大型數據
3. **圖像緩存** - JavaScript 自動緩存背景圖像
4. **ROI 優化** - 只發送必要的 ROI 數據

### 性能監控
```rust
// 在 draw_roi.rs 中測量
let start = std::time::Instant::now();
draw_ctx.read().redraw(None);
tracing::info!("Redraw took: {:?}ms", start.elapsed().as_millis());
```

## 文檔定位

- **詳細技術** → REFACTORING.md
- **API 參考** → CANVAS_API_GUIDE.md
- **架構圖** → ARCHITECTURE.md
- **完成清單** → COMPLETION_CHECKLIST.md
- **項目報告** → COMPLETION_REPORT.md

## 支援的平台

| 平台 | 支援 | Canvas | 備註 |
|------|------|--------|------|
| Web (WASM) | ✅ | ✅ | 完全支援 |
| Linux | ✅ | ❌ | 編譯成功，無 canvas |
| macOS | ✅ | ❌ | 編譯成功，無 canvas |
| Windows | ✅ | ❌ | 編譯成功，無 canvas |
| iOS | ⚠️ | ⚠️ | 未測試 |
| Android | ⚠️ | ⚠️ | 未測試 |

## 版本相容性

- **Dioxus**: 0.7.1+
- **Rust**: 1.70+
- **wasm-bindgen**: 0.2.90+
- **serde**: 1.0+

## 聯繫與反饋

如有任何問題或改進建議，請參考項目文檔或提交 issue。

---

**最後更新**: 2026-04-09
**狀態**: ✅ 生產就緒

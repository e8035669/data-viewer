
function applyTheme(theme) {
    const html = document.documentElement;
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

    if (theme === "dark" || (theme === "auto" && prefersDark)) {
        html.dataset.theme = "dark";
    } else {
        html.dataset.theme = "light";
    }
}

// 切換主題時儲存並套用
function setTheme(theme) {
    localStorage.setItem("theme", theme);
    applyTheme(theme);
}

function getTheme() {
    return localStorage.getItem("theme") ?? "auto";
}

// 初始化（頁面載入時）
const saved = localStorage.getItem("theme") ?? "auto";
applyTheme(saved);

// auto 模式下監聽 OS 偏好變化
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const current = localStorage.getItem("theme") ?? "auto";
    if (current === "auto") applyTheme("auto");
});

const ThemeProto = {
    setTheme(theme) {
        setTheme(theme)
    },

    getTheme() {
        return getTheme()
    },
};

window.themeProvider = Object.create(ThemeProto);
use crate::models::{Endpoints, Projects};
use crate::{views::global::HeaderContext, Route};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::fa_solid_icons, Icon};

/// The Home page component that will be rendered when the current route is `[Route::Home]`.
///
/// This page gives a short introduction to the Data Viewer application and guides first-time
/// visitors through the initial setup steps: creating an endpoint and then adding a project.
#[component]
pub fn Home() -> Element {
    use_effect(|| {
        consume_context::<HeaderContext>().set_title("Home");
    });

    let endpoints = use_context::<Signal<Endpoints>>();
    let projects = use_context::<Signal<Projects>>();

    let endpoint_count = endpoints.read().len();
    let project_count = projects.read().len();

    let has_endpoints = endpoint_count > 0;
    let has_projects = project_count > 0;
    let is_fully_configured = has_endpoints && has_projects;

    rsx! {
        div { class: "container mx-auto py-8 px-4 max-w-6xl space-y-10",
            // 頂部歡迎區
            div { class: "relative overflow-hidden rounded-xl border border-slate-200 dark:border-zinc-800 bg-slate-50 dark:bg-zinc-800/30 p-8 text-slate-900 dark:text-slate-100",
                div { class: "relative z-10 space-y-3.5",
                    span { class: "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-slate-100 dark:bg-zinc-800 text-slate-900 dark:text-slate-100 border border-slate-200 dark:border-zinc-700",
                        if is_fully_configured {
                            "🎉 全功能就緒"
                        } else {
                            "🚀 新手設定中"
                        }
                    }
                    h1 { class: "text-3xl font-extrabold tracking-tight text-slate-900 dark:text-slate-100 sm:text-4xl", "Data Viewer" }
                    p { class: "text-sm text-slate-500 dark:text-slate-400 max-w-2xl leading-relaxed",
                        "專案、裝置與感測器歷史資料的一站式管理平台。在這裡您可以輕鬆串接多個端點，即時匯入專案並監控裝置狀態。"
                    }
                }
            }

            // 引導狀態摘要
            div { class: "space-y-6",
                div { class: "flex items-center justify-between border-b border-slate-200 dark:border-zinc-800 pb-4",
                    div {
                        h2 { class: "text-2xl font-bold tracking-tight text-slate-900 dark:text-slate-100",
                            if is_fully_configured { "🎉 設定完成！開始探索您的資料" } else { "🔌 快速上手引導" }
                        }
                        p { class: "text-slate-500 dark:text-slate-400 mt-1",
                            if is_fully_configured {
                                "您已成功設定了 {endpoint_count} 個 API 端點並建立了 {project_count} 個專案。"
                            } else {
                                "完成以下兩個簡單步驟，即可啟用您的資料監控流程。"
                            }
                        }
                    }
                    if is_fully_configured {
                        span { class: "px-3 py-1 rounded-full text-sm font-medium bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800",
                            "100% Configured"
                        }
                    } else {
                        span { class: "px-3 py-1 rounded-full text-sm font-medium bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300 border border-amber-200 dark:border-amber-800",
                            if has_endpoints { "50% Completed" } else { "0% Completed" }
                        }
                    }
                }

                // 步驟卡片排版
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                    // 步驟 1: Configure Endpoint
                    div {
                        class: "group relative flex flex-col justify-between p-6 rounded-xl border",
                        class: if has_endpoints {
                            "bg-emerald-50/50 dark:bg-emerald-950/10 border-emerald-200 dark:border-emerald-800/50"
                        } else {
                            "bg-white dark:bg-zinc-900 border-blue-200 dark:border-blue-800/30 hover:border-blue-500"
                        },
                        div { class: "space-y-4",
                            div { class: "flex items-center justify-between",
                                span {
                                    class: "text-xs font-semibold px-2.5 py-1 rounded-md tracking-wider uppercase",
                                    class: if has_endpoints { "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300" } else { "bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-300" },
                                    "Step 1"
                                }
                                if has_endpoints {
                                    span { class: "flex items-center gap-1 text-sm text-emerald-600 dark:text-emerald-400 font-medium",
                                        Icon { icon: fa_solid_icons::FaCheck, width: 14, height: 14 }
                                        "已設定"
                                    }
                                }
                            }
                            div { class: "flex items-start gap-3",
                                div {
                                    class: "p-2.5 rounded-lg text-white mt-1",
                                    class: if has_endpoints { "bg-emerald-500" } else { "bg-blue-600" },
                                    Icon { icon: fa_solid_icons::FaLink, width: 20, height: 20 }
                                }
                                div {
                                    h3 { class: "text-lg font-semibold text-slate-900 dark:text-slate-100", "設定 API 連線端點" }
                                    p { class: "text-sm text-slate-500 dark:text-slate-400 mt-1.5 leading-relaxed",
                                        "端點是用於取得感測器與專案資料的 API 伺服器網址。請在此設定您的 General 或 Edge 伺服器 URL。"
                                    }
                                    if has_endpoints {
                                        p { class: "text-xs text-emerald-600 dark:text-emerald-400 mt-2 font-medium",
                                            "✓ 目前已啟用 {endpoint_count} 個活躍的端點連線"
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "mt-6 pt-4 border-t border-dashed border-slate-200 dark:border-zinc-800",
                            Link {
                                to: Route::EndpointView {},
                                class: if has_endpoints {
                                    "inline-flex items-center justify-center w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-900 dark:text-slate-100 border border-slate-200 dark:border-zinc-700"
                                } else {
                                    "inline-flex items-center justify-center w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-blue-600 hover:bg-blue-700 text-white"
                                },
                                if has_endpoints { "管理連線端點" } else { "設定您的端點 ➔" }
                            }
                        }
                    }

                    // 步驟 2: Manage Projects
                    div {
                        class: "group relative flex flex-col justify-between p-6 rounded-xl border",
                        class: if !has_endpoints {
                            "bg-slate-100/40 dark:bg-zinc-800/40 border-slate-200 dark:border-zinc-800/50 text-slate-400 dark:text-slate-600 cursor-not-allowed"
                        } else if has_projects {
                            "bg-emerald-50/50 dark:bg-emerald-950/10 border-emerald-200 dark:border-emerald-800/50"
                        } else {
                            "bg-white dark:bg-zinc-900 border-purple-200 dark:border-purple-800/30 hover:border-purple-500"
                        },
                        div { class: "space-y-4",
                            div { class: "flex items-center justify-between",
                                span {
                                    class: "text-xs font-semibold px-2.5 py-1 rounded-md tracking-wider uppercase",
                                    class: if !has_endpoints {
                                        "bg-slate-100 dark:bg-zinc-800 text-slate-400 dark:text-slate-600"
                                    } else if has_projects {
                                        "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300"
                                    } else {
                                        "bg-purple-100 text-purple-800 dark:bg-purple-950 dark:text-purple-300"
                                    },
                                    "Step 2"
                                }
                                if has_projects {
                                    span { class: "flex items-center gap-1 text-sm text-emerald-600 dark:text-emerald-400 font-medium",
                                        Icon { icon: fa_solid_icons::FaCheck, width: 14, height: 14 }
                                        "已建立"
                                    }
                                }
                            }
                            div { class: "flex items-start gap-3",
                                div {
                                    class: "p-2.5 rounded-lg text-white mt-1",
                                    class: if !has_endpoints {
                                        "bg-slate-300 dark:bg-zinc-750 text-slate-400 dark:text-slate-600"
                                    } else if has_projects {
                                        "bg-emerald-500"
                                    } else {
                                        "bg-purple-600"
                                    },
                                    Icon { icon: fa_solid_icons::FaCirclePlus, width: 20, height: 20 }
                                }
                                div {
                                    h3 { class: "text-lg font-semibold text-slate-900 dark:text-slate-100", "建立或匯入專案" }
                                    p { class: "text-sm text-slate-500 dark:text-slate-400 mt-1.5 leading-relaxed",
                                        "您可以手動新增專案，或透過已設定的端點直接即時匯入現有專案。專案將包含其所屬的全部感測裝置。"
                                    }
                                    if !has_endpoints {
                                        p { class: "text-xs text-amber-600 dark:text-amber-400 mt-2 font-medium",
                                            "⚠️ 請先完成步驟一以啟用此步驟"
                                        }
                                    } else if has_projects {
                                        p { class: "text-xs text-emerald-600 dark:text-emerald-400 mt-2 font-medium",
                                            "✓ 目前已成功匯入並管理 {project_count} 個專案"
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "mt-6 pt-4 border-t border-dashed border-slate-200 dark:border-zinc-800",
                            if !has_endpoints {
                                button {
                                    disabled: true,
                                    class: "w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-slate-100 dark:bg-zinc-800 text-slate-400 dark:text-slate-600 cursor-not-allowed border border-slate-200 dark:border-zinc-800",
                                    "等待步驟一完成"
                                }
                            } else {
                                Link {
                                    to: Route::ProjectsView {},
                                    class: if has_projects {
                                        "inline-flex items-center justify-center w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-slate-100 hover:bg-slate-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-slate-900 dark:text-slate-100 border border-slate-200 dark:border-zinc-700"
                                    } else {
                                        "inline-flex items-center justify-center w-full px-4 py-2.5 rounded-lg text-sm font-medium bg-purple-600 hover:bg-purple-700 text-white"
                                    },
                                    if has_projects { "進入專案管理" } else { "建立您的專案 ➔" }
                                }
                            }
                        }
                    }
                }
            }

            // 完成狀態下的特別提示（例如：快速功能通道）
            if is_fully_configured {
                div { class: "p-6 rounded-xl border border-indigo-100 dark:border-indigo-950 bg-indigo-50/20 dark:bg-indigo-950/10 space-y-4",
                    h3 { class: "text-lg font-semibold text-indigo-900 dark:text-indigo-200 flex items-center gap-2",
                        Icon { icon: fa_solid_icons::FaRocket, width: 18, height: 18 }
                        "⚡ 快速通道 (Quick Access)"
                    }
                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                        Link {
                            to: Route::ProjectsView {},
                            class: "flex items-center justify-between p-4 rounded-xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 hover:border-indigo-500 group",
                            div {
                                div { class: "font-semibold text-slate-900 dark:text-slate-100", "專案儀表板" }
                                div { class: "text-xs text-slate-500 dark:text-slate-400 mt-0.5", "管理裝置與感測器" }
                            }
                            Icon { icon: fa_solid_icons::FaArrowRight, width: 14, height: 14, class: "text-slate-400" }
                        }
                        Link {
                            to: Route::ActiveMonitorView {},
                            class: "flex items-center justify-between p-4 rounded-xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 hover:border-indigo-500 group",
                            div {
                                div { class: "font-semibold text-slate-900 dark:text-slate-100", "裝置監控" }
                                div { class: "text-xs text-slate-500 dark:text-slate-400 mt-0.5", "即時監控裝置運作狀態" }
                            }
                            Icon { icon: fa_solid_icons::FaArrowRight, width: 14, height: 14, class: "text-slate-400" }
                        }
                        Link {
                            to: Route::DrawRoiPage {},
                            class: "flex items-center justify-between p-4 rounded-xl bg-white dark:bg-zinc-900 border border-slate-200 dark:border-zinc-800 hover:border-indigo-500 group",
                            div {
                                div { class: "font-semibold text-slate-900 dark:text-slate-100", "ROI 繪圖工具" }
                                div { class: "text-xs text-slate-500 dark:text-slate-400 mt-0.5", "在影像中編輯與標註感興趣區" }
                            }
                            Icon { icon: fa_solid_icons::FaArrowRight, width: 14, height: 14, class: "text-slate-400" }
                        }
                    }
                }
            }

            // 功能亮點與架構區（重新排版成精美卡片網格）
            div { class: "space-y-6",
                h2 { class: "text-2xl font-bold tracking-tight text-slate-900 dark:text-slate-100 border-b border-slate-200 dark:border-zinc-800 pb-3", "💡 系統全功能概覽" }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-6",
                    div { class: "p-5 rounded-xl border border-slate-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 space-y-3",
                        div { class: "w-10 h-10 rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 flex items-center justify-center font-bold text-lg", "✓" }
                        h3 { class: "font-bold text-slate-900 dark:text-slate-100 text-lg", "專案與裝置管理" }
                        p { class: "text-sm text-slate-500 dark:text-slate-400 leading-relaxed",
                            "支援手動建立、刪除或從 Credentials (General/Edge) 匯入專案。完整顯示裝置清單、感測器最新數值等核心資料。"
                        }
                    }
                    div { class: "p-5 rounded-xl border border-slate-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 space-y-3",
                        div { class: "w-10 h-10 rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 flex items-center justify-center font-bold text-lg", "✓" }
                        h3 { class: "font-bold text-slate-900 dark:text-slate-100 text-lg", "歷史資料與影像回溯" }
                        p { class: "text-sm text-slate-500 dark:text-slate-400 leading-relaxed",
                            "針對數值感測器提供歷史變化趨勢；針對影像感測器則支援歷史圖庫、基於時間區間與篩選條件的影像精確回溯。"
                        }
                    }
                    div { class: "p-5 rounded-xl border border-slate-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 space-y-3",
                        div { class: "w-10 h-10 rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 flex items-center justify-center font-bold text-lg", "✓" }
                        h3 { class: "font-bold text-slate-900 dark:text-slate-100 text-lg", "即時活躍監控與 ROI" }
                        p { class: "text-sm text-slate-500 dark:text-slate-400 leading-relaxed",
                            "支援多專案裝置運作監控、自動定時重新整理、警報規則設定。內建 ROI 標註工具，便於對影像設定警戒感興趣區。"
                        }
                    }
                }
            }

            // 安全與存儲提示
            div { class: "flex items-start gap-3.5 p-4 rounded-xl border border-blue-100 dark:border-blue-950 bg-blue-50/30 dark:bg-blue-950/10 text-blue-800 dark:text-blue-200",
                div { class: "mt-0.5 text-blue-600 dark:text-blue-400",
                    Icon { icon: fa_solid_icons::FaCheck, width: 18, height: 18 }
                }
                div { class: "space-y-1",
                    h4 { class: "font-semibold text-blue-950 dark:text-blue-100 text-sm", "🔒 資料與安全隱私提示" }
                    p { class: "text-xs text-blue-900/80 dark:text-blue-300 leading-relaxed",
                        "您的所有設定（包括 Endpoints 連線、專案詳情、Credentials 認證）均持久儲存於您的瀏覽器本機（LocalStorage/Storage Crate）。我們不會收集或將您的任何隱私金鑰與資料上傳至雲端，請放心使用。"
                    }
                }
            }
        }
    }
}

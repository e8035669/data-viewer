use dioxus::{logger::tracing, prelude::*};

use crate::components::date_picker::{DatePicker, DateRangePicker};

#[component]
pub fn TestPage() -> Element {
    let mut date_range = use_signal(|| None);

    let mut start_date = use_signal(|| None);
    let mut end_date = use_signal(|| None);

    rsx! {
        p { "This is a test page" }

        DateRangePicker {
            selected_range: date_range,
            on_range_change: move |v| {
                if v != date_range() {
                    tracing::info!("on_range_change: {v:?}");
                    date_range.set(v)
                }
            },
        }
        // 如果接了on_range_change事件，會網頁卡住壞掉，也不是寫入與取值的無限循環，無解
        p { "Selected range: {date_range():?}" }

        div { class: "flex gap-4",
            DatePicker {
                selected_date: start_date,
                on_value_change: move |v| { start_date.set(v) },
            }
            DatePicker {
                selected_date: end_date,
                on_value_change: move |v| { end_date.set(v) },
            }
        }
        p { "Start date: {start_date():?}, End date: {end_date():?}" }

        div { class: "h-96" }

    }
}

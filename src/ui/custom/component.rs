use crate::components::{
    button::{Button, ButtonVariant},
    dropdown_menu::{DropdownMenuContent, DropdownMenuTrigger},
    input::Input,
};
use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::dropdown_menu::DropdownMenuContentProps;
use dioxus_primitives::merge_attributes;

#[css_module("/src/components/input/style.css")]
struct InputStyles;

/// Bridges `components::input::Input` so every call site keeps the `dx-input` styling
/// even if the underlying component's attribute/event API changes upstream.
#[component]
pub fn DxInput(
    oninput: Option<EventHandler<FormEvent>>,
    onchange: Option<EventHandler<FormEvent>>,
    oninvalid: Option<EventHandler<FormEvent>>,
    onselect: Option<EventHandler<SelectionEvent>>,
    onselectionchange: Option<EventHandler<SelectionEvent>>,
    onfocus: Option<EventHandler<FocusEvent>>,
    onblur: Option<EventHandler<FocusEvent>>,
    onfocusin: Option<EventHandler<FocusEvent>>,
    onfocusout: Option<EventHandler<FocusEvent>>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    onkeypress: Option<EventHandler<KeyboardEvent>>,
    onkeyup: Option<EventHandler<KeyboardEvent>>,
    onwheel: Option<EventHandler<WheelEvent>>,
    oncompositionstart: Option<EventHandler<CompositionEvent>>,
    oncompositionupdate: Option<EventHandler<CompositionEvent>>,
    oncompositionend: Option<EventHandler<CompositionEvent>>,
    oncopy: Option<EventHandler<ClipboardEvent>>,
    oncut: Option<EventHandler<ClipboardEvent>>,
    onpaste: Option<EventHandler<ClipboardEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = input)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(input {
        class: InputStyles::dx_input,
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        Input {
            oninput: move |e| _ = oninput.map(|callback| callback(e)),
            onchange: move |e| _ = onchange.map(|callback| callback(e)),
            oninvalid: move |e| _ = oninvalid.map(|callback| callback(e)),
            onselect: move |e| _ = onselect.map(|callback| callback(e)),
            onselectionchange: move |e| _ = onselectionchange.map(|callback| callback(e)),
            onfocus: move |e| _ = onfocus.map(|callback| callback(e)),
            onblur: move |e| _ = onblur.map(|callback| callback(e)),
            onfocusin: move |e| _ = onfocusin.map(|callback| callback(e)),
            onfocusout: move |e| _ = onfocusout.map(|callback| callback(e)),
            onkeydown: move |e| _ = onkeydown.map(|callback| callback(e)),
            onkeypress: move |e| _ = onkeypress.map(|callback| callback(e)),
            onkeyup: move |e| _ = onkeyup.map(|callback| callback(e)),
            onwheel: move |e| _ = onwheel.map(|callback| callback(e)),
            oncompositionstart: move |e| _ = oncompositionstart.map(|callback| callback(e)),
            oncompositionupdate: move |e| _ = oncompositionupdate.map(|callback| callback(e)),
            oncompositionend: move |e| _ = oncompositionend.map(|callback| callback(e)),
            oncopy: move |e| _ = oncopy.map(|callback| callback(e)),
            oncut: move |e| _ = oncut.map(|callback| callback(e)),
            onpaste: move |e| _ = onpaste.map(|callback| callback(e)),
            attributes: merged,
            {children}
        }
    }
}

/// Bridges `components::dropdown_menu::DropdownMenuTrigger` so every trigger keeps the
/// ghost/shadow-none button styling. `children` is cloned per call since the `r#as`
/// callback must be `FnMut`, not `FnOnce`.
#[component]
pub fn DxDropdownMenuTrigger(children: Element) -> Element {
    rsx! {
        DropdownMenuTrigger {
            r#as: move |attributes| rsx! {
                Button { attributes, variant: ButtonVariant::Ghost, class: "shadow-none!", {children.clone()} }
            },
        }
    }
}

#[component]
pub fn DxDropdownMenuContent(props: DropdownMenuContentProps) -> Element {
    let base = attributes!(div {
        class: "left-auto! right-0! origin-top-right!",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    rsx! {
        DropdownMenuContent { id: props.id, attributes: merged, {props.children} }
    }
}

//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals.

// NOTE: the original template included a `Hero` component for the home page, but the
// homepage has since been rewritten and no longer uses this component. The `hero.rs`
// file can be deleted if it is no longer needed.

pub mod card;
pub mod button;
pub mod skeleton;
pub mod sheet;
pub mod separator;
pub mod tooltip;
pub mod sidebar;
pub mod dropdown_menu;
pub mod label;
pub mod input;
pub mod dialog;
pub mod toast;
pub mod select;
pub mod radio_group;
pub mod textarea;

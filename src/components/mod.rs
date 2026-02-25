//! The components module contains all shared components for our app. Components are the building blocks of dioxus apps.
//! They can be used to defined common UI elements like buttons, forms, and modals. In this template, we define a Hero
//! component  to be used in our app.

mod hero;
pub use hero::Hero;
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

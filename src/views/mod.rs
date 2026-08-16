//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//!
//! The [`Home`] and [`Blog`] components will be rendered when the current route is [`Route::Home`] or [`Route::Blog`] respectively.
//!
//!
//! The [`Navbar`] component will be rendered on all pages of our app since every page is under the layout. The layout defines
//! a common wrapper around all child routes.

mod home;
pub use home::Home;

mod blog;
pub use blog::Blog;

mod navbar;
pub use navbar::Navbar;

mod sensor;
pub use sensor::TestRule1;

mod endpoints;
pub use endpoints::{EndpointView, Storage, Storage2};

mod projects;
pub use projects::ProjectsView;

mod global;
pub use global::Providers;

mod monitors;
pub use monitors::{ActiveMonitorView, MonitorProjectPage, MonitorProjectSelectPage};

mod draw_roi;
pub use draw_roi::DrawRoiPage;

mod import;
pub use import::{AuthInfosPage, ProjectImportPage};

mod sensor_v2;
pub use sensor_v2::{
    DeviceAttr, DeviceSensors, ProjectDevices, ProjectLayout, SensorAttr, SensorHistory,
};

mod attribute_batch;
pub use attribute_batch::ProjectAttributeBatch;

mod attribute_overview;
pub use attribute_overview::ProjectAttributeOverview;

mod darkmode;
pub use darkmode::ThemeProvider;

mod preference;
pub use preference::PreferencePage;

mod export;
pub use export::{ExportDataPage, ExportSnapshotsPage};

mod testing;
pub use testing::TestPage;

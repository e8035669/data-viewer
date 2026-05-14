use dioxus::prelude::*;
use dioxus_sdk_storage::{use_synced_storage, LocalStorage};

use crate::models::{AuthInfos, Endpoints, Projects};

pub fn use_count_persistent() -> Signal<i32> {
    use_synced_storage::<LocalStorage, _>("count".to_string(), || 0)
}

pub fn use_endpoints_persistent() -> Signal<Endpoints> {
    use_synced_storage::<LocalStorage, _>("endpoints".to_string(), || Endpoints::new())
}

pub fn use_project_persistence() -> Signal<Projects> {
    use_synced_storage::<LocalStorage, _>("projects".to_string(), || Projects::new())
}

pub fn use_auth_infos_persistent() -> Signal<AuthInfos> {
    use_synced_storage::<LocalStorage, _>("auth_infos".to_string(), || AuthInfos::new())
}

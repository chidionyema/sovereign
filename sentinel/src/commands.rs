use crate::Sentinel;
use serde::Serialize;

#[derive(Serialize)]
pub struct PermissionStatus {
    pub accessibility: bool,
    pub screen_recording: bool,
}

pub async fn permission_status() -> PermissionStatus {
    PermissionStatus {
        accessibility: crate::permissions::accessibility_granted(),
        screen_recording: crate::permissions::screen_recording_granted(),
    }
}

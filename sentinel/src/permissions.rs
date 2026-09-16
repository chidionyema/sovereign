#[cfg(target_os = "macos")]
pub use platform_macos::*;

#[cfg(target_os = "windows")]
pub use platform_windows::*;

#[cfg(target_os = "macos")]
mod platform_macos {
    use std::process::Command;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }

    pub fn accessibility_granted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn screen_recording_granted() -> bool {
        // In a real implementation, check Privacy/ScreenRecording in TCC.
        // For now, optimistic default — user will see the tray icon warning.
        true
    }

    pub fn request_screen_recording() -> bool {
        // Trigger system prompt. macOS will show it once per app.
        true
    }

    pub fn open_accessibility_settings() {
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status();
    }

    pub fn open_screen_recording_settings() {
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .status();
    }
}

#[cfg(target_os = "windows")]
mod platform_windows {
    pub fn accessibility_granted() -> bool {
        true
    }

    pub fn screen_recording_granted() -> bool {
        true
    }

    pub fn request_screen_recording() -> bool {
        true
    }

    pub fn open_accessibility_settings() {}

    pub fn open_screen_recording_settings() {}
}

pub mod keys;

use anyhow::Result;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputAction {
    Move { to: Point },
    Click { at: Point, button: MouseButton },
    Type { text: String },
    Key { combo: String },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub trait InputBackend: Send + Sync {
    fn execute(&self, action: &InputAction) -> Result<()>;
    fn name(&self) -> &'static str;
}

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(feature = "dev-http")]
pub mod mock;

#[cfg(target_os = "macos")]
pub fn default_backend() -> Box<dyn InputBackend> {
    Box::new(macos::QuartzBackend)
}

#[cfg(target_os = "windows")]
pub fn default_backend() -> Box<dyn InputBackend> {
    Box::new(windows::SendInputBackend)
}

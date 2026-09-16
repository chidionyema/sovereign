#[cfg(feature = "dev-http")]
use super::{InputAction, InputBackend};
use anyhow::Result;
use std::sync::Mutex;

#[derive(Default)]
pub struct MockInputBackend {
    pub calls: Mutex<Vec<InputAction>>,
}

#[cfg(feature = "dev-http")]
impl InputBackend for MockInputBackend {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn execute(&self, action: &InputAction) -> Result<()> {
        self.calls.lock().unwrap().push(action.clone());
        Ok(())
    }
}

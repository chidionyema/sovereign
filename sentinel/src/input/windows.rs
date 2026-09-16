#[cfg(target_os = "windows")]
use super::*;
use crate::input::keys::{Key, KeyCombo};
use anyhow::{anyhow, Result};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct SendInputBackend;

impl SendInputBackend {
    fn screen_size() -> (f64, f64) {
        unsafe {
            (
                GetSystemMetrics(SM_CXSCREEN) as f64,
                GetSystemMetrics(SM_CYSCREEN) as f64,
            )
        }
    }

    fn to_abs(x: f64, y: f64) -> (i32, i32) {
        let (sw, sh) = Self::screen_size();
        (((x / sw) * 65535.0) as i32, ((y / sh) * 65535.0) as i32)
    }
}

fn vk(k: &Key) -> Result<u16> {
    use Key::*;
    Ok(match k {
        Char(c) if c.is_ascii_alphanumeric() => c.to_ascii_uppercase() as u16,
        Char(' ') => 0x20,
        Char(';') => 0xBA,
        Char('=') => 0xBB,
        Char(',') => 0xBC,
        Char('-') => 0xBD,
        Char('.') => 0xBE,
        Char('/') => 0xBF,
        Char('`') => 0xC0,
        Char('[') => 0xDB,
        Char('\\') => 0xDC,
        Char(']') => 0xDD,
        Char('\'') => 0xDE,
        Char(other) => anyhow::bail!("no VK for {other:?}"),
        Return => 0x0D,
        Tab => 0x09,
        Space => 0x20,
        Backspace => 0x08,
        Escape => 0x1B,
        ForwardDelete => 0x2E,
        Left => 0x25,
        Up => 0x26,
        Right => 0x27,
        Down => 0x28,
        Home => 0x24,
        End => 0x23,
        PageUp => 0x21,
        PageDown => 0x22,
        F(n) if (1..=24).contains(n) => 0x70 + (*n as u16 - 1),
        F(n) => anyhow::bail!("win F{n} unsupported"),
    })
}

unsafe fn send_vk(vk_code: u16, up: bool) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk_code),
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}

unsafe fn post_combo_windows(combo: &KeyCombo) -> Result<()> {
    let kc = vk(&combo.key)?;
    let m = &combo.modifiers;

    let mods_down: Vec<u16> = [
        (m.cmd, 0x5B),
        (m.ctrl, 0xA2),
        (m.alt, 0xA4),
        (m.shift, 0xA0),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .map(|(_, v)| *v)
    .collect();

    for &v in &mods_down {
        send_vk(v, false);
    }
    send_vk(kc, false);
    send_vk(kc, true);
    for &v in mods_down.iter().rev() {
        send_vk(v, true);
    }
    Ok(())
}

impl InputBackend for SendInputBackend {
    fn name(&self) -> &'static str {
        "sendinput"
    }

    fn execute(&self, action: &InputAction) -> Result<()> {
        unsafe {
            match action {
                InputAction::Move { to } => {
                    let (ax, ay) = Self::to_abs(to.x, to.y);
                    let input = INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: ax,
                                dy: ay,
                                mouseData: 0,
                                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                }
                InputAction::Click { at, button } => {
                    let (ax, ay) = Self::to_abs(at.x, at.y);
                    let (down, up) = match button {
                        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
                        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
                        MouseButton::Middle => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
                    };
                    for flags in [
                        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | down,
                        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | up,
                    ] {
                        let input = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: ax,
                                    dy: ay,
                                    mouseData: 0,
                                    dwFlags: flags,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        };
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                }
                InputAction::Type { text } => {
                    for ch in text.encode_utf16(CharArrayBuffer::new()) {
                        for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
                            let input = INPUT {
                                r#type: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: VIRTUAL_KEY(0),
                                        wScan: ch,
                                        dwFlags: flags,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            };
                            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                        }
                    }
                }
                InputAction::Key { combo } => {
                    let c =
                        KeyCombo::parse(combo).map_err(|e| anyhow!("bad combo {combo:?}: {e}"))?;
                    post_combo_windows(&c)?;
                }
            }
        }
        Ok(())
    }
}

struct CharArrayBuffer([u16; 2]);
impl CharArrayBuffer {
    fn new() -> Self {
        Self([0; 2])
    }
}
impl std::convert::AsRef<[u16]> for CharArrayBuffer {
    fn as_ref(&self) -> &[u16] {
        &self.0
    }
}

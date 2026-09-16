#[cfg(target_os = "macos")]
use super::*;
use crate::input::keys::{Key, KeyCombo, Modifiers};
use anyhow::{anyhow, Result};
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

pub struct QuartzBackend;

impl QuartzBackend {
    fn source() -> Result<CGEventSource> {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow!("CGEventSource creation failed (Accessibility permission?)"))
    }
}

fn keycode(k: &Key) -> Result<u16> {
    use Key::*;
    Ok(match k {
        Char(c) => match c.to_ascii_lowercase() {
            'a' => 0x00,
            's' => 0x01,
            'd' => 0x02,
            'f' => 0x03,
            'h' => 0x04,
            'g' => 0x05,
            'z' => 0x06,
            'x' => 0x07,
            'c' => 0x08,
            'v' => 0x09,
            'b' => 0x0B,
            'q' => 0x0C,
            'w' => 0x0D,
            'e' => 0x0E,
            'r' => 0x0F,
            'y' => 0x10,
            't' => 0x11,
            '1' => 0x12,
            '2' => 0x13,
            '3' => 0x14,
            '4' => 0x15,
            '6' => 0x16,
            '5' => 0x17,
            '=' => 0x18,
            '9' => 0x19,
            '7' => 0x1A,
            '-' => 0x1B,
            '8' => 0x1C,
            '0' => 0x1D,
            ']' => 0x1E,
            'o' => 0x1F,
            'u' => 0x20,
            '[' => 0x21,
            'i' => 0x22,
            'p' => 0x23,
            'l' => 0x25,
            'j' => 0x26,
            '\'' => 0x27,
            'k' => 0x28,
            ';' => 0x29,
            '\\' => 0x2A,
            ',' => 0x2B,
            '/' => 0x2C,
            'n' => 0x2D,
            'm' => 0x2E,
            '.' => 0x2F,
            '`' => 0x32,
            other => anyhow::bail!("no mac keycode for {other:?}"),
        },
        Return => 0x24,
        Tab => 0x30,
        Space => 0x31,
        Backspace => 0x33,
        Escape => 0x35,
        ForwardDelete => 0x75,
        Left => 0x7B,
        Right => 0x7C,
        Down => 0x7D,
        Up => 0x7E,
        Home => 0x73,
        End => 0x77,
        PageUp => 0x74,
        PageDown => 0x79,
        F(n) => match n {
            1 => 0x7A,
            2 => 0x78,
            3 => 0x63,
            4 => 0x76,
            5 => 0x60,
            6 => 0x61,
            7 => 0x62,
            8 => 0x64,
            9 => 0x65,
            10 => 0x6D,
            11 => 0x67,
            12 => 0x6F,
            _ => anyhow::bail!("mac F{n} unsupported"),
        },
    })
}

fn flags(m: &Modifiers) -> core_graphics::event::CGEventFlags {
    use core_graphics::event::CGEventFlags;
    let mut f = CGEventFlags::CGEventFlagNull;
    if m.cmd {
        f |= CGEventFlags::CGEventFlagCommand;
    }
    if m.shift {
        f |= CGEventFlags::CGEventFlagShift;
    }
    if m.ctrl {
        f |= CGEventFlags::CGEventFlagControl;
    }
    if m.alt {
        f |= CGEventFlags::CGEventFlagAlternate;
    }
    if m.r#fn {
        f |= CGEventFlags::CGEventFlagSecondaryFn;
    }
    f
}

fn post_combo(src: &CGEventSource, combo: &KeyCombo) -> Result<()> {
    let kc = keycode(&combo.key)?;
    let fl = flags(&combo.modifiers);
    for down in [true, false] {
        let ev = CGEvent::new_keyboard_event(src.clone(), kc, down)
            .map_err(|_| anyhow!("new_keyboard_event"))?;
        ev.set_flags(fl);
        ev.post(CGEventTapLocation::HID);
    }
    Ok(())
}

impl InputBackend for QuartzBackend {
    fn name(&self) -> &'static str {
        "quartz"
    }

    fn execute(&self, action: &InputAction) -> Result<()> {
        let src = Self::source()?;
        match action {
            InputAction::Move { to } => {
                let ev = CGEvent::new_mouse_event(
                    src.clone(),
                    CGEventType::MouseMoved,
                    CGPoint::new(to.x, to.y),
                    CGMouseButton::Left,
                )
                .map_err(|_| anyhow!("new_mouse_event"))?;
                ev.post(CGEventTapLocation::HID);
            }
            InputAction::Click { at, button } => {
                let (down, up, mb) = match button {
                    MouseButton::Left => (
                        CGEventType::LeftMouseDown,
                        CGEventType::LeftMouseUp,
                        CGMouseButton::Left,
                    ),
                    MouseButton::Right => (
                        CGEventType::RightMouseDown,
                        CGEventType::RightMouseUp,
                        CGMouseButton::Right,
                    ),
                    MouseButton::Middle => (
                        CGEventType::OtherMouseDown,
                        CGEventType::OtherMouseUp,
                        CGMouseButton::Center,
                    ),
                };
                let pt = CGPoint::new(at.x, at.y);
                let d = CGEvent::new_mouse_event(src.clone(), down, pt, mb)
                    .map_err(|_| anyhow!("down"))?;
                let u = CGEvent::new_mouse_event(src, up, pt, mb).map_err(|_| anyhow!("up"))?;
                d.post(CGEventTapLocation::HID);
                u.post(CGEventTapLocation::HID);
            }
            InputAction::Type { text } => {
                for ch in text.chars() {
                    let s = ch.to_string();
                    let down = CGEvent::new_keyboard_event(src.clone(), 0, true)
                        .map_err(|_| anyhow!("kbd down"))?;
                    down.set_string(&s);
                    down.post(CGEventTapLocation::HID);
                    let up = CGEvent::new_keyboard_event(src.clone(), 0, false)
                        .map_err(|_| anyhow!("kbd up"))?;
                    up.set_string(&s);
                    up.post(CGEventTapLocation::HID);
                }
            }
            InputAction::Key { combo } => {
                let c = KeyCombo::parse(combo)
                    .map_err(|e| anyhow!("bad combo {combo:?}: {e}"))?;
                post_combo(&src, &c)?;
            }
        }
        Ok(())
    }
}

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Modifiers {
    #[serde(default)]
    pub cmd: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub r#fn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum Key {
    Char(char),
    Return,
    Tab,
    Space,
    Backspace,
    Escape,
    ForwardDelete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl KeyCombo {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            bail!("empty key combo");
        }
        let mut parts: Vec<&str> = s.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            bail!("no tokens in key combo: {s:?}");
        }
        let key_tok = parts.pop().unwrap();
        let mut modifiers = Modifiers::default();
        for m in parts {
            match m.to_ascii_lowercase().as_str() {
                "cmd" | "command" | "meta" | "super" | "win" => modifiers.cmd = true,
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "opt" | "option" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                "fn" | "function" => modifiers.r#fn = true,
                other => bail!("unknown modifier {other:?} in {s:?}"),
            }
        }
        let key = Self::parse_key(key_tok)?;
        Ok(Self { modifiers, key })
    }

    fn parse_key(tok: &str) -> Result<Key> {
        let mut chars = tok.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            return Ok(Key::Char(c));
        }
        let lower = tok.to_ascii_lowercase();
        Ok(match lower.as_str() {
            "return" | "enter" => Key::Return,
            "tab" => Key::Tab,
            "space" => Key::Space,
            "backspace" | "delete" => Key::Backspace,
            "forwarddelete" | "fwddel" => Key::ForwardDelete,
            "escape" | "esc" => Key::Escape,
            "left" | "arrowleft" => Key::Left,
            "right" | "arrowright" => Key::Right,
            "up" | "arrowup" => Key::Up,
            "down" | "arrowdown" => Key::Down,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" | "pgup" => Key::PageUp,
            "pagedown" | "pgdn" => Key::PageDown,
            other => {
                if let Some(n) = other.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                    if (1..=24).contains(&n) {
                        return Ok(Key::F(n));
                    }
                }
                bail!("unknown key {tok:?}")
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_combos() {
        let c = KeyCombo::parse("cmd+shift+T").unwrap();
        assert_eq!(
            c.modifiers,
            Modifiers {
                cmd: true,
                shift: true,
                ..Default::default()
            }
        );
        assert_eq!(c.key, Key::Char('T'));

        let c = KeyCombo::parse("ctrl+alt+del").unwrap();
        assert_eq!(c.key, Key::Backspace);

        let c = KeyCombo::parse("f5").unwrap();
        assert_eq!(c.key, Key::F(5));
    }

    #[test]
    fn rejects_junk() {
        assert!(KeyCombo::parse("cmd+frobnicate").is_err());
        assert!(KeyCombo::parse("").is_err());
        assert!(KeyCombo::parse("fnord+a").is_err());
    }
}

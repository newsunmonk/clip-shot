//! 앱 설정 모델 + 영속화 + 하위호환 마이그레이션.
//!
//! 모든 신규 필드는 `#[serde(default = ...)]`로 기본값을 주어, 옛 settings.json에
//! 필드가 없어도 안전하게 디코딩된다(= 마이그레이션). 로드 시 `sanitize`로 범위를 보정한다.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_HISTORY_LIMIT: usize = 30;
pub const MIN_HISTORY_LIMIT: usize = 1;
pub const MAX_HISTORY_LIMIT: usize = 200;

fn t() -> bool {
    true
}
fn default_history_limit() -> usize {
    DEFAULT_HISTORY_LIMIT
}
fn default_lang() -> String {
    "ko".into()
}
fn default_theme() -> String {
    "system".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    #[default]
    Png,
    Jpg,
}

impl ImageFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpg => "jpg",
        }
    }
}

// macOS 기본 스크린샷 단축키와 동일하게 맞춘다(기본 캡처를 끄면 ClipShot이 대신 동작).
fn ds_region() -> String {
    "Cmd+Shift+4".into()
}
fn ds_window() -> String {
    "Cmd+Shift+5".into()
}
fn ds_display() -> String {
    "Cmd+Shift+3".into()
}
fn ds_all() -> String {
    "Cmd+Shift+6".into()
}
fn ds_history() -> String {
    "Cmd+Shift+1".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcuts {
    #[serde(default = "ds_region")]
    pub region: String,
    #[serde(default = "ds_window")]
    pub window: String,
    #[serde(default = "ds_display")]
    pub display: String,
    #[serde(default = "ds_all")]
    pub all_displays: String,
    #[serde(default = "ds_history")]
    pub history: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Shortcuts {
            region: ds_region(),
            window: ds_window(),
            display: ds_display(),
            all_displays: ds_all(),
            history: ds_history(),
        }
    }
}

impl Shortcuts {
    /// (액션 키, 가속기 문자열) 목록. 단축키 등록/충돌 검사에 사용.
    pub fn pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("region", self.region.clone()),
            ("window", self.window.clone()),
            ("display", self.display.clone()),
            ("all_displays", self.all_displays.clone()),
            ("history", self.history.clone()),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "t")]
    pub autostart: bool,
    #[serde(default = "t")]
    pub hotkeys_enabled: bool,
    #[serde(default)]
    pub shortcuts: Shortcuts,
    #[serde(default = "t")]
    pub show_toast: bool,
    #[serde(default = "t")]
    pub shutter_sound: bool,
    #[serde(default = "t")]
    pub flash_effect: bool,
    #[serde(default)]
    pub image_format: ImageFormat,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default = "default_lang")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            autostart: true,
            hotkeys_enabled: true,
            shortcuts: Shortcuts::default(),
            show_toast: true,
            shutter_sound: true,
            flash_effect: true,
            image_format: ImageFormat::Png,
            history_limit: DEFAULT_HISTORY_LIMIT,
            language: default_lang(),
            theme: default_theme(),
        }
    }
}

impl Settings {
    /// 값 범위를 보정한다(히스토리 개수 clamp). 잘못된 영속값에 대한 방어.
    pub fn sanitized(mut self) -> Self {
        self.history_limit = self
            .history_limit
            .clamp(MIN_HISTORY_LIMIT, MAX_HISTORY_LIMIT);
        if self.language != "ko" && self.language != "en" {
            self.language = default_lang();
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = default_theme();
        }
        self
    }
}

pub fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

/// settings.json을 읽는다. 파일이 없거나 손상되면 기본값으로 폴백한다.
pub fn load(data_dir: &Path) -> Settings {
    let path = settings_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Settings>(&text)
            .unwrap_or_default()
            .sanitized(),
        Err(_) => Settings::default(),
    }
}

pub fn save(data_dir: &Path, settings: &Settings) -> Result<(), String> {
    let path = settings_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let s = Settings::default();
        assert_eq!(s.history_limit, 30);
        assert!(s.hotkeys_enabled);
        assert_eq!(s.image_format, ImageFormat::Png);
        assert_eq!(s.language, "ko");
        assert_eq!(s.theme, "system");
        assert!(s.shutter_sound);
        assert_eq!(s.shortcuts.region, "Cmd+Shift+4");
    }

    #[test]
    fn empty_json_migrates_to_all_defaults() {
        // 옛 설정(빈 객체)도 모든 신규 필드가 기본값으로 채워져야 한다.
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn partial_json_keeps_known_and_defaults_rest() {
        let json = r#"{ "historyLimit": 10, "shutterSound": true }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.history_limit, 10);
        assert!(s.shutter_sound);
        // 나머지는 기본값
        assert!(s.hotkeys_enabled);
        assert_eq!(s.shortcuts, Shortcuts::default());
    }

    #[test]
    fn partial_shortcuts_fill_missing() {
        let json = r#"{ "shortcuts": { "region": "CmdOrCtrl+Alt+R" } }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.shortcuts.region, "CmdOrCtrl+Alt+R");
        assert_eq!(s.shortcuts.window, "Cmd+Shift+5");
    }

    #[test]
    fn sanitize_clamps_history_limit() {
        let high = Settings {
            history_limit: 9999,
            ..Settings::default()
        };
        assert_eq!(high.sanitized().history_limit, MAX_HISTORY_LIMIT);
        let low = Settings {
            history_limit: 0,
            ..Settings::default()
        };
        assert_eq!(low.sanitized().history_limit, MIN_HISTORY_LIMIT);
    }

    #[test]
    fn sanitize_resets_unknown_language() {
        let s = Settings {
            language: "fr".into(),
            ..Settings::default()
        };
        assert_eq!(s.sanitized().language, "ko");
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let s = Settings {
            history_limit: 50,
            shutter_sound: true,
            language: "en".into(),
            ..Settings::default()
        };
        let text = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(s, back);
    }
}

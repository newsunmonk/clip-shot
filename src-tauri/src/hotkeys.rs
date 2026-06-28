//! 전역 단축키 가속기 문자열 검증 + 충돌 검사.
//!
//! 실제 등록은 `tauri-plugin-global-shortcut`이 담당하지만, 사용자가 설정 UI에서 바꾼
//! 단축키가 유효한지/서로 충돌하지 않는지 검증하는 순수 로직을 여기 둔다.
//! 파싱 규칙은 플러그인이 쓰는 `global_hotkey` 크레이트와 동일하게 맞춘다.

use tauri_plugin_global_shortcut::Shortcut;

/// 가속기 문자열이 파싱 가능한지 검증한다. 예: "CmdOrCtrl+Shift+2".
pub fn validate(accel: &str) -> Result<(), String> {
    if accel.trim().is_empty() {
        return Err("단축키가 비어 있습니다".into());
    }
    accel
        .parse::<Shortcut>()
        .map(|_| ())
        .map_err(|e| format!("잘못된 단축키 '{accel}': {e}"))
}

/// 같은 가속기를 여러 액션이 쓰면 충돌. 충돌한 (가속기, 액션들)을 정규화 키로 묶어 반환한다.
/// 비교는 대소문자/순서 차이를 흡수하기 위해 파싱된 표준형(Debug)을 키로 쓴다.
pub fn find_conflicts(pairs: &[(&str, String)]) -> Vec<(String, Vec<String>)> {
    use std::collections::BTreeMap;
    let mut by_key: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (action, accel) in pairs {
        // 파싱 실패한 항목은 충돌 검사에서 제외(별도 validate에서 잡힘).
        if let Ok(sc) = accel.parse::<Shortcut>() {
            by_key
                .entry(format!("{sc:?}"))
                .or_default()
                .push(action.to_string());
        }
    }
    by_key
        .into_iter()
        .filter(|(_, actions)| actions.len() > 1)
        .collect()
}

/// 단축키 묶음 전체가 유효하고 충돌이 없는지 검사한다.
pub fn validate_all(pairs: &[(&str, String)]) -> Result<(), String> {
    for (action, accel) in pairs {
        validate(accel).map_err(|e| format!("[{action}] {e}"))?;
    }
    let conflicts = find_conflicts(pairs);
    if !conflicts.is_empty() {
        let desc: Vec<String> = conflicts
            .iter()
            .map(|(_, actions)| actions.join(", "))
            .collect();
        return Err(format!("단축키 충돌: {}", desc.join(" / ")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_accelerators_pass() {
        assert!(validate("CmdOrCtrl+Shift+2").is_ok());
        assert!(validate("Alt+Shift+R").is_ok());
        assert!(validate("CommandOrControl+Shift+H").is_ok());
    }

    #[test]
    fn invalid_accelerators_fail() {
        assert!(validate("").is_err());
        assert!(validate("NotAKey+++").is_err());
        assert!(validate("Shift+Shift+Shift").is_err());
    }

    #[test]
    fn no_conflict_for_distinct_shortcuts() {
        let pairs = vec![
            ("region", "CmdOrCtrl+Shift+2".to_string()),
            ("window", "CmdOrCtrl+Shift+W".to_string()),
        ];
        assert!(find_conflicts(&pairs).is_empty());
        assert!(validate_all(&pairs).is_ok());
    }

    #[test]
    fn detects_identical_shortcuts() {
        let pairs = vec![
            ("region", "CmdOrCtrl+Shift+2".to_string()),
            ("display", "CmdOrCtrl+Shift+2".to_string()),
        ];
        let conflicts = find_conflicts(&pairs);
        assert_eq!(conflicts.len(), 1);
        assert!(validate_all(&pairs).is_err());
    }

    #[test]
    fn conflict_detection_is_order_and_case_insensitive() {
        // 같은 조합을 다른 표기로 적어도 충돌로 잡아야 한다.
        let pairs = vec![
            ("a", "CmdOrCtrl+Shift+R".to_string()),
            ("b", "shift+cmdorctrl+r".to_string()),
        ];
        assert_eq!(find_conflicts(&pairs).len(), 1);
    }
}

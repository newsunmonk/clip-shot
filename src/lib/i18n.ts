// 경량 i18n. 한국어 기본 + 영어. `t(lang, key)`로 조회한다.
export type Lang = "ko" | "en";

type Dict = Record<string, { ko: string; en: string }>;

const dict: Dict = {
  "app.title": { ko: "클립샷", en: "ClipShot" },
  "app.subtitle": {
    ko: "캡처 → 클립보드 즉시 복사 + 최근 기록",
    en: "Capture → instant clipboard + recent history",
  },
  "tab.history": { ko: "히스토리", en: "History" },
  "tab.settings": { ko: "설정", en: "Settings" },

  "mode.region": { ko: "영역 캡처", en: "Region" },
  "mode.window": { ko: "창 캡처", en: "Window" },
  "mode.display": { ko: "디스플레이 캡처", en: "Display" },
  "mode.all": { ko: "모든 디스플레이", en: "All displays" },

  "name.region": { ko: "영역", en: "Region" },
  "name.window": { ko: "창", en: "Window" },
  "name.display": { ko: "디스플레이", en: "Display" },
  "name.all": { ko: "모든 디스플레이", en: "All displays" },

  "history.empty": {
    ko: "아직 캡처가 없습니다. 위 버튼이나 단축키로 캡처해 보세요.",
    en: "No captures yet. Use the buttons above or a shortcut.",
  },
  "history.copy": { ko: "복사", en: "Copy" },
  "history.export": { ko: "내보내기", en: "Export" },
  "history.delete": { ko: "삭제", en: "Delete" },
  "history.clear": { ko: "전체 비우기", en: "Clear all" },
  "history.clearConfirm": { ko: "모든 캡처 기록을 지울까요?", en: "Clear all captures?" },
  "history.count": { ko: "개 보관 중", en: "kept" },

  "toast.copied": { ko: "클립보드에 복사됨", en: "Copied to clipboard" },
  "toast.captured": { ko: "캡처 완료 · 클립보드 복사됨", en: "Captured · copied to clipboard" },
  "toast.exported": { ko: "파일로 저장됨", en: "Exported to file" },
  "toast.deleted": { ko: "삭제됨", en: "Deleted" },

  "settings.general": { ko: "일반", en: "General" },
  "settings.autostart": { ko: "시작 프로그램으로 등록", en: "Launch at login" },
  "settings.hotkeysEnabled": { ko: "전역 단축키 사용", en: "Enable global shortcuts" },
  "settings.effects": { ko: "효과", en: "Effects" },
  "settings.showToast": { ko: "캡처 완료 알림 표시", en: "Show capture toast" },
  "settings.shutterSound": { ko: "셔터 소리", en: "Shutter sound" },
  "settings.flashEffect": { ko: "캡처 플래시", en: "Capture flash" },
  "settings.storage": { ko: "저장", en: "Storage" },
  "settings.imageFormat": { ko: "내보내기 형식", en: "Export format" },
  "settings.historyLimit": { ko: "히스토리 보관 개수", en: "History limit" },
  "settings.language": { ko: "언어", en: "Language" },
  "settings.theme": { ko: "테마", en: "Theme" },
  "theme.system": { ko: "시스템", en: "System" },
  "theme.light": { ko: "라이트", en: "Light" },
  "theme.dark": { ko: "다크", en: "Dark" },
  "settings.shortcuts": { ko: "단축키", en: "Shortcuts" },
  "settings.shortcutHint": {
    ko: "예: CmdOrCtrl+Shift+2 (Mac은 Cmd, Windows는 Ctrl)",
    en: "e.g. CmdOrCtrl+Shift+2 (Cmd on Mac, Ctrl on Windows)",
  },
  "settings.save": { ko: "저장", en: "Save" },
  "settings.saved": { ko: "저장되었습니다", en: "Saved" },
  "shortcut.history": { ko: "히스토리 열기", en: "Open history" },

  "picker.title": { ko: "캡처할 창 선택", en: "Choose a window" },
  "picker.empty": { ko: "캡처 가능한 창이 없습니다.", en: "No capturable windows." },
  "picker.displayTitle": { ko: "캡처할 디스플레이 선택", en: "Choose a display" },
  "display.primary": { ko: "주 디스플레이", en: "Primary" },

  "preview.copy": { ko: "복사", en: "Copy" },
  "preview.export": { ko: "내보내기", en: "Export" },
  "preview.close": { ko: "닫기", en: "Close" },

  "shortcut.record": { ko: "클릭 후 키 입력", en: "Click, then press keys" },
  "shortcut.recording": { ko: "키를 누르세요…", en: "Press keys…" },
  "settings.hotkeyNote": {
    ko: "앱을 닫아도 트레이에 남아 단축키만으로 캡처됩니다.",
    en: "Stays in the tray after closing — capture by shortcut anytime.",
  },

  "overlay.hint": {
    ko: "드래그하여 영역 선택 · ESC 취소",
    en: "Drag to select · ESC to cancel",
  },
  "perm.title": { ko: "화면 기록 권한이 필요합니다", en: "Screen Recording permission required" },
  "perm.desc": {
    ko: "다른 앱 창까지 캡처하려면 화면 기록 권한을 허용해야 합니다. 허용한 뒤 앱을 재시작하세요.",
    en: "Grant Screen Recording to capture other apps' windows, then restart the app.",
  },
  "perm.request": { ko: "권한 요청", en: "Request" },
  "perm.settings": { ko: "시스템 설정 열기", en: "Open Settings" },
  "perm.restart": { ko: "재시작", en: "Restart" },
  "perm.recheck": { ko: "다시 확인", en: "Re-check" },

  "settings.nativeShortcuts": { ko: "맥 기본 캡처 대체", en: "Replace macOS capture" },
  "settings.nativeNote": {
    ko: "같은 단축키로 ClipShot이 대신 뜨게 하려면, 시스템 설정에서 맥 기본 스크린샷 단축키를 끄세요. 기본 단축키는 macOS와 동일하게 맞춰져 있습니다.",
    en: "To make ClipShot take over the same keys, turn off macOS native screenshot shortcuts in System Settings. Defaults already match macOS.",
  },
  "settings.openKeyboard": { ko: "키보드 단축키 설정 열기", en: "Open keyboard shortcuts" },
  "settings.nativeDisable": { ko: "맥 기본 캡처 단축키 끄기", en: "Turn off macOS capture shortcuts" },
  "settings.nativeRestore": { ko: "되돌리기", en: "Restore" },
  "settings.nativeApplied": { ko: "바로 적용되었습니다", en: "Applied instantly" },

  "common.cancel": { ko: "취소", en: "Cancel" },
  "common.refresh": { ko: "새로고침", en: "Refresh" },
  "error.prefix": { ko: "오류", en: "Error" },
  "error.permission": {
    ko: "화면 기록 권한이 필요할 수 있습니다. (시스템 설정 → 개인정보 보호 → 화면 기록)",
    en: "Screen recording permission may be required. (System Settings → Privacy → Screen Recording)",
  },
};

export function t(lang: Lang, key: string): string {
  const entry = dict[key];
  if (!entry) return key;
  return entry[lang] ?? entry.ko;
}

export function modeName(lang: Lang, mode: string): string {
  const key =
    mode === "all" ? "name.all" : mode === "window" ? "name.window" : mode === "region" ? "name.region" : "name.display";
  return t(lang, key);
}

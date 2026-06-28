//! macOS 화면 기록(Screen Recording) 권한 확인/요청.
//!
//! 권한이 없으면 xcap 캡처가 에러 없이 "바탕화면 + 자기 앱 창"만 담긴 저하 이미지를 돌려준다.
//! 그래서 캡처 전 권한을 확실히 요청/확인해야 한다. (다른 OS에서는 항상 true)

#[cfg(target_os = "macos")]
mod imp {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
        // 비공개 API: 시스템 심볼릭 단축키(스크린샷 등)를 로그아웃 없이 즉시 on/off.
        fn CGSSetSymbolicHotKeyEnabled(hot_key: i32, enabled: bool) -> i32;
    }

    /// 이미 권한이 있는지(프롬프트 없이 조회).
    pub fn has() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    /// 권한을 요청한다. 미허용 상태면 시스템 권한 다이얼로그를 띄우고 앱을 목록에 등록한다.
    pub fn request() -> bool {
        unsafe { CGRequestScreenCaptureAccess() }
    }

    // macOS 기본 스크린샷 심볼릭 단축키 ID:
    // 28 전체→파일, 29 전체→클립보드, 30 영역→파일, 31 영역→클립보드, 184 스크린샷 옵션(Cmd+Shift+5)
    const SCREENSHOT_HOTKEYS: [i32; 5] = [28, 29, 30, 31, 184];

    /// 맥 기본 스크린샷 단축키를 즉시 on/off (로그아웃 불필요).
    pub fn set_screenshot_hotkeys(enabled: bool) {
        unsafe {
            for id in SCREENSHOT_HOTKEYS {
                CGSSetSymbolicHotKeyEnabled(id, enabled);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn has() -> bool {
        true
    }
    pub fn request() -> bool {
        true
    }
    pub fn set_screenshot_hotkeys(_enabled: bool) {}
}

pub use imp::{has, request, set_screenshot_hotkeys};

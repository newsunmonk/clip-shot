//! 클립샷(ClipShot) — 화면 캡처 → 클립보드 즉시 복사 + 최근 N개 히스토리.
//!
//! 트레이 상주 + 전역 단축키로 어디서든 캡처한다. 영역 선택은 투명 오버레이 창에서 처리한다.

mod capture;
mod clipboard;
mod history;
mod hotkeys;
mod macperm;
mod settings;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use history::HistoryEntry;
use settings::Settings;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub struct AppState {
    pub data_dir: PathBuf,
    pub settings: Mutex<Settings>,
    // 영역 선택용으로 모니터마다 "고정(freeze)"한 캡처(물리 픽셀). id → 이미지.
    pub overlay_bgs: Mutex<HashMap<u32, image::RgbaImage>>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 캡처 결과 공통 마무리: 클립보드 write + 히스토리 저장 + 프론트 알림.
fn finalize(app: &AppHandle, mode: &str, img: image::RgbaImage) -> Result<HistoryEntry, String> {
    clipboard::write_image(app, &img)?;
    let state = app.state::<AppState>();
    let limit = state.settings.lock().unwrap().history_limit;
    let entry = history::add(&state.data_dir, &img, mode, now_ms(), limit)?;
    let _ = app.emit("captured", &entry);
    Ok(entry)
}

// ───────────────────────── 캡처 명령 ─────────────────────────

#[tauri::command]
fn capture_display(app: AppHandle, id: Option<u32>) -> Result<HistoryEntry, String> {
    let img = capture::capture_display(id)?;
    finalize(&app, "display", img)
}

#[tauri::command]
fn list_displays() -> Result<Vec<capture::DisplayInfo>, String> {
    capture::list_displays()
}

/// RgbaImage를 가로 max 픽셀 썸네일 PNG data URL로 변환.
fn thumb_data_url(img: &image::RgbaImage, max: u32) -> Result<String, String> {
    let w = img.width().clamp(1, max);
    let h = ((img.height() as f64) * (w as f64) / (img.width().max(1) as f64))
        .round()
        .max(1.0) as u32;
    let thumb = image::imageops::thumbnail(img, w, h);
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(thumb)
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    Ok(format!("data:image/png;base64,{b64}"))
}

/// 디스플레이 라이브 미리보기(선택 창에서 어떤 화면인지 보여주기 위함).
#[tauri::command]
fn display_preview(id: u32) -> Result<String, String> {
    let img = capture::capture_display(Some(id))?;
    thumb_data_url(&img, 520)
}

/// 창 라이브 미리보기.
#[tauri::command]
fn window_preview(id: u32) -> Result<String, String> {
    let img = capture::capture_window(id)?;
    thumb_data_url(&img, 520)
}

#[tauri::command]
fn capture_all_displays(app: AppHandle) -> Result<HistoryEntry, String> {
    let img = capture::capture_all_displays()?;
    finalize(&app, "all", img)
}

#[tauri::command]
fn capture_window(app: AppHandle, id: u32) -> Result<HistoryEntry, String> {
    let img = capture::capture_window(id)?;
    finalize(&app, "window", img)
}

#[tauri::command]
fn list_windows() -> Result<Vec<capture::WindowInfo>, String> {
    capture::list_windows()
}

/// 열려 있는 모든 영역 오버레이 창을 닫는다.
fn close_overlays(app: &AppHandle) {
    for (label, win) in app.webview_windows() {
        if label.starts_with("overlay") {
            let _ = win.close();
        }
    }
}

/// 오버레이에서 받은 **전역 논리 좌표**로 영역 캡처를 확정한다.
/// 오버레이 표시 시점에 고정(freeze)해 둔 이미지에서 잘라내므로 정확하고 빠르다.
#[tauri::command]
fn capture_region(
    app: AppHandle,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<HistoryEntry, String> {
    let cx = x + w as i32 / 2;
    let cy = y + h as i32 / 2;

    // 중심이 속한 모니터를 찾아, 고정해 둔 이미지에서 물리 픽셀로 크롭.
    let displays = capture::list_displays()?;
    let target = displays.iter().find(|d| {
        cx >= d.x && cx < d.x + d.width as i32 && cy >= d.y && cy < d.y + d.height as i32
    });

    let cropped = {
        let state = app.state::<AppState>();
        let bgs = state.overlay_bgs.lock().unwrap();
        match target.and_then(|d| bgs.get(&d.id).map(|img| (d, img))) {
            Some((d, img)) => {
                let lx = ((x - d.x).max(0) as f32 * d.scale).round() as u32;
                let ly = ((y - d.y).max(0) as f32 * d.scale).round() as u32;
                let pw = (w as f32 * d.scale).round() as u32;
                let ph = (h as f32 * d.scale).round() as u32;
                Some(capture::crop(img, lx, ly, pw, ph)?)
            }
            None => None,
        }
    };

    close_overlays(&app);
    app.state::<AppState>().overlay_bgs.lock().unwrap().clear();

    let img = match cropped {
        Some(i) => i,
        None => {
            // 고정 이미지가 없으면(예외) 라이브로 폴백.
            std::thread::sleep(std::time::Duration::from_millis(120));
            capture::capture_region_global(x, y, w, h)?
        }
    };
    finalize(&app, "region", img)
}

/// 창 호버 강조용 사각형 목록.
#[tauri::command]
fn list_window_rects() -> Result<Vec<capture::WindowRect>, String> {
    capture::list_window_rects()
}

/// 영역 오버레이의 돋보기(loupe)용으로 자기 모니터 고정 화면을 JPEG data URL로 반환(가벼움).
#[tauri::command]
fn overlay_bg(app: AppHandle, id: u32) -> Option<String> {
    let state = app.state::<AppState>();
    let bgs = state.overlay_bgs.lock().unwrap();
    let img = bgs.get(&id)?;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img.clone())
        .to_rgb8()
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    Some(format!("data:image/jpeg;base64,{b64}"))
}

/// 호버한 창을 즉시 캡처(오버레이를 닫은 뒤 실제 창 캡처).
#[tauri::command]
fn commit_window(app: AppHandle, id: u32) -> Result<HistoryEntry, String> {
    close_overlays(&app);
    app.state::<AppState>().overlay_bgs.lock().unwrap().clear();
    std::thread::sleep(std::time::Duration::from_millis(120));
    let img = capture::capture_window(id)?;
    finalize(&app, "window", img)
}

/// 호버한 디스플레이를 즉시 캡처(고정해 둔 이미지 사용).
#[tauri::command]
fn commit_display(app: AppHandle, id: u32) -> Result<HistoryEntry, String> {
    let frozen = {
        let state = app.state::<AppState>();
        let bgs = state.overlay_bgs.lock().unwrap();
        bgs.get(&id).cloned()
    };
    close_overlays(&app);
    app.state::<AppState>().overlay_bgs.lock().unwrap().clear();
    let img = match frozen {
        Some(i) => i,
        None => capture::capture_display(Some(id))?,
    };
    finalize(&app, "display", img)
}

#[tauri::command]
fn cancel_region(app: AppHandle) {
    close_overlays(&app);
    app.state::<AppState>().overlay_bgs.lock().unwrap().clear();
}

/// 모니터마다 투명 오버레이 창을 하나씩 띄운다(단일 창은 macOS에서 다중 모니터를
/// 덮지 못하므로). 각 창은 자신의 모니터 좌표에 위치하고, 프론트가 창 위치로 전역 좌표를 계산한다.
/// 영역/창/디스플레이 캡처를 위한 통합 오버레이를 모니터마다 띄운다.
/// 표시 직전 각 모니터를 고정(freeze)해 배경으로 쓰며, mode는 창 label에 담는다.
#[tauri::command]
fn open_capture_overlay(app: AppHandle, mode: String) -> Result<(), String> {
    close_overlays(&app);
    let displays = capture::list_displays()?;

    // region/display는 오버레이가 캡처에 섞이지 않도록 표시 직전 각 모니터를 고정(freeze).
    // window는 개별 창을 실시간 캡처하므로 freeze 불필요(더 빠름).
    if mode != "window" {
        let mut bgs = HashMap::new();
        for d in &displays {
            if let Ok(img) = capture::capture_display(Some(d.id)) {
                bgs.insert(d.id, img);
            }
        }
        *app.state::<AppState>().overlay_bgs.lock().unwrap() = bgs;
    } else {
        app.state::<AppState>().overlay_bgs.lock().unwrap().clear();
    }

    for d in displays {
        let label = format!("overlay-{mode}-{}", d.id);
        let win = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
            .title("영역 선택")
            .position(d.x as f64, d.y as f64)
            .inner_size(d.width as f64, d.height as f64)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false) // 앱을 활성화하지 않음(Stage Manager가 창을 치우지 않도록)
            .accept_first_mouse(true) // 포커스 없이도 첫 클릭부터 반응
            .visible_on_all_workspaces(true)
            .visible(true)
            .build()
            .map_err(|e| e.to_string())?;
        let _ = win;
    }
    Ok(())
}

// ───────────────────────── 히스토리 명령 ─────────────────────────

#[tauri::command]
fn list_history(app: AppHandle) -> Vec<HistoryEntry> {
    let state = app.state::<AppState>();
    history::load_index(&state.data_dir)
}

/// 썸네일을 data URL(base64)로 반환. 프론트가 그리드에 표시.
#[tauri::command]
fn history_thumb(app: AppHandle, id: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    let path = history::history_dir(&state.data_dir).join(format!("{id}.thumb.png"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// 원본 풀 이미지를 data URL로 반환. 큰 미리보기에 사용.
#[tauri::command]
fn history_full(app: AppHandle, id: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    let path = history::full_image_path(&state.data_dir, &id).ok_or("원본을 찾을 수 없습니다")?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// 히스토리 항목을 다시 클립보드로 복사.
#[tauri::command]
fn copy_history(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let path = history::full_image_path(&state.data_dir, &id).ok_or("원본을 찾을 수 없습니다")?;
    let img = image::open(&path).map_err(|e| e.to_string())?.to_rgba8();
    clipboard::write_image(&app, &img)
}

#[tauri::command]
fn delete_history(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    history::delete(&state.data_dir, &id)
}

#[tauri::command]
fn clear_history(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    history::clear(&state.data_dir)
}

/// 히스토리 항목을 파일로 내보낸다. 저장 경로는 프론트의 파일 다이얼로그가 고른다.
#[tauri::command]
fn export_history(app: AppHandle, id: String, dest: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let format = state.settings.lock().unwrap().image_format;
    history::export(&state.data_dir, &id, std::path::Path::new(&dest), format)
}

#[tauri::command]
fn default_export_name(app: AppHandle, id: String) -> String {
    let state = app.state::<AppState>();
    let ext = state.settings.lock().unwrap().image_format.ext();
    format!("clipshot-{id}.{ext}")
}

// ───────────────────────── 설정 명령 ─────────────────────────

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let current = state.settings.lock().unwrap().clone();
    current
}

#[tauri::command]
fn save_settings(app: AppHandle, new_settings: Settings) -> Result<(), String> {
    let cleaned = new_settings.sanitized();
    // 단축키 유효성/충돌 검사
    hotkeys::validate_all(&cleaned.shortcuts.pairs())?;

    {
        let state = app.state::<AppState>();
        settings::save(&state.data_dir, &cleaned)?;
        *state.settings.lock().unwrap() = cleaned.clone();
    }

    apply_autostart(&app, cleaned.autostart);
    apply_hotkeys(&app);
    let _ = app.emit("settings-changed", &cleaned);
    Ok(())
}

#[tauri::command]
fn show_main(app: AppHandle) {
    show_main_window(&app);
}

// ───────────────────────── 권한 ─────────────────────────

#[tauri::command]
fn screen_permission() -> bool {
    macperm::has()
}

#[tauri::command]
fn request_screen_permission() -> bool {
    macperm::request()
}

#[tauri::command]
fn open_screen_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
    }
}

/// macOS 키보드 단축키 설정(스크린샷 항목)을 연다 — 기본 캡처 단축키를 끄도록 안내.
#[tauri::command]
fn open_keyboard_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.keyboard?Shortcuts")
            .spawn();
    }
}

#[tauri::command]
fn restart_app(app: AppHandle) {
    app.restart();
}

/// macOS 기본 스크린샷 단축키(전체/영역/옵션) 즉시 on/off. 끄면 같은 단축키를 ClipShot이 차지한다.
/// 비공개 CGS API라 로그아웃 없이 바로 적용된다. (다른 OS에서는 no-op)
#[tauri::command]
fn set_native_screenshots(enabled: bool) {
    macperm::set_screenshot_hotkeys(enabled);
}

/// 현재 로그인 항목(시작 프로그램) 등록 여부.
#[tauri::command]
fn autostart_status(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

// ───────────────────────── 보조 ─────────────────────────

fn show_main_window(app: &AppHandle) {
    // 메인 창을 보일 땐 일반 앱(dock 표시)으로 전환해 확실히 앞으로 가져온다.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn apply_autostart(app: &AppHandle, enable: bool) {
    let mgr = app.autolaunch();
    let _ = if enable { mgr.enable() } else { mgr.disable() };
}

/// 현재 설정에 맞춰 전역 단축키를 재등록한다.
fn apply_hotkeys(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let state = app.state::<AppState>();
    let s = state.settings.lock().unwrap();
    if !s.hotkeys_enabled {
        return;
    }
    for (_action, accel) in s.shortcuts.pairs() {
        if let Ok(sc) = accel.parse::<Shortcut>() {
            let _ = gs.register(sc);
        }
    }
}

/// 눌린 단축키를 액션으로 변환해 프론트로 전달한다.
fn dispatch_hotkey(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    let s = state.settings.lock().unwrap();
    if !s.hotkeys_enabled {
        return;
    }
    let pressed = format!("{shortcut:?}");
    let action = s.shortcuts.pairs().into_iter().find_map(|(a, accel)| {
        accel
            .parse::<Shortcut>()
            .ok()
            .filter(|sc| format!("{sc:?}") == pressed)
            .map(|_| a)
    });
    drop(s);
    if let Some(action) = action {
        let _ = app.emit("hotkey-action", action);
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let region = MenuItem::with_id(app, "region", "영역 캡처", true, None::<&str>)?;
    let window = MenuItem::with_id(app, "window", "창 캡처", true, None::<&str>)?;
    let display = MenuItem::with_id(app, "display", "디스플레이 캡처", true, None::<&str>)?;
    let all = MenuItem::with_id(app, "all_displays", "모든 디스플레이 캡처", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let open_history = MenuItem::with_id(app, "open_history", "히스토리 열기", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(app, "open_settings", "설정 열기", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &region,
            &window,
            &display,
            &all,
            &sep1,
            &open_history,
            &open_settings,
            &sep2,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("클립샷")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "region" | "window" | "display" | "all_displays" => {
                let _ = app.emit("hotkey-action", event.id.as_ref());
            }
            "open_history" => {
                show_main_window(app);
                let _ = app.emit("navigate", "history");
            }
            "open_settings" => {
                show_main_window(app);
                let _ = app.emit("navigate", "settings");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        dispatch_hotkey(app, shortcut);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = handle
                .path()
                .app_data_dir()
                .expect("app_data_dir 사용 불가");
            std::fs::create_dir_all(&data_dir).ok();
            let loaded = settings::load(&data_dir);
            app.manage(AppState {
                data_dir,
                settings: Mutex::new(loaded.clone()),
                overlay_bgs: Mutex::new(HashMap::new()),
            });

            build_tray(&handle)?;
            apply_autostart(&handle, loaded.autostart);
            apply_hotkeys(&handle);
            // 화면 기록 권한이 없으면 시작 시 한 번 요청(앱을 권한 목록에 등록 + 다이얼로그).
            if !macperm::has() {
                macperm::request();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 메인 창은 닫지 않고 숨겨 트레이/단축키가 계속 살아 있게 한다.
                // 숨길 땐 메뉴바 전용(Accessory)으로 바꿔 dock 아이콘을 없애고,
                // 단축키 캡처 시 메인 창이 튀어나오지 않게 한다.
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window
                        .app_handle()
                        .set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            capture_display,
            list_displays,
            display_preview,
            window_preview,
            capture_all_displays,
            capture_window,
            list_windows,
            capture_region,
            cancel_region,
            open_capture_overlay,
            list_window_rects,
            overlay_bg,
            commit_window,
            commit_display,
            list_history,
            history_thumb,
            history_full,
            copy_history,
            delete_history,
            clear_history,
            export_history,
            default_export_name,
            get_settings,
            save_settings,
            show_main,
            screen_permission,
            request_screen_permission,
            open_screen_settings,
            open_keyboard_settings,
            restart_app,
            set_native_screenshots,
            autostart_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

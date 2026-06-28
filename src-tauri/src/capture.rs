//! 화면 캡처 코어. xcap으로 모니터/창 단위 캡처 + 영역 크롭 + 다중 모니터 합성.
//!
//! 좌표 규약(이 Mac에서 검증): `Monitor::x/y/width/height`는 **논리 좌표(points)**,
//! `capture_image()`는 **물리 픽셀**(= 논리 × scale_factor)을 돌려준다.
//!
//! - 디스플레이: 특정(또는 주) 모니터 1개
//! - 모든 디스플레이: 모든 모니터를 논리 배치대로 한 장에 합성(`compose`)
//! - 창: id로 특정 창
//! - 영역: 전체 데스크탑을 덮는 오버레이에서 받은 **전역 논리 좌표**로 해당 모니터를 찾아 크롭
//!
//! OS 권한(특히 macOS 화면 기록)이 필요하며 런타임 동작이라 캡처 자체는 단위 테스트 대상이 아니다.
//! 대신 순수 합성/크롭 로직(`compose`, `crop`)을 테스트한다.

use image::{imageops, GenericImage, RgbaImage};
use serde::Serialize;
use xcap::{Monitor, Window};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app_name: String,
}

/// 창의 전역 논리 좌표 사각형(호버 강조용). z가 클수록 앞쪽.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowRect {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub z: i32,
    pub app_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,  // 논리
    pub height: u32, // 논리
    pub scale: f32,
    pub is_primary: bool,
}

fn all_monitors() -> Result<Vec<Monitor>, String> {
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    if monitors.is_empty() {
        return Err("모니터를 찾을 수 없습니다".into());
    }
    Ok(monitors)
}

fn primary_monitor() -> Result<Monitor, String> {
    let monitors = all_monitors()?;
    for m in &monitors {
        if m.is_primary().unwrap_or(false) {
            return Ok(m.clone());
        }
    }
    Ok(monitors.into_iter().next().unwrap())
}

pub fn list_displays() -> Result<Vec<DisplayInfo>, String> {
    let mut out = Vec::new();
    for m in all_monitors()? {
        out.push(DisplayInfo {
            id: m.id().map_err(|e| e.to_string())?,
            name: m.name().unwrap_or_default(),
            x: m.x().map_err(|e| e.to_string())?,
            y: m.y().map_err(|e| e.to_string())?,
            width: m.width().map_err(|e| e.to_string())?,
            height: m.height().map_err(|e| e.to_string())?,
            scale: m.scale_factor().unwrap_or(1.0),
            is_primary: m.is_primary().unwrap_or(false),
        });
    }
    Ok(out)
}

pub fn capture_display(id: Option<u32>) -> Result<RgbaImage, String> {
    let monitor = match id {
        Some(want) => {
            let monitors = all_monitors()?;
            monitors
                .into_iter()
                .find(|m| m.id().map(|i| i == want).unwrap_or(false))
                .ok_or("디스플레이를 찾을 수 없습니다")?
        }
        None => primary_monitor()?,
    };
    monitor.capture_image().map_err(|e| e.to_string())
}

// ───────────────────── 다중 모니터 합성(순수 로직) ─────────────────────

/// 합성 입력 타일: 논리 위치/크기 + scale + 물리 이미지.
pub struct Tile {
    pub lx: i32,
    pub ly: i32,
    pub lw: u32,
    pub lh: u32,
    pub img: RgbaImage,
}

/// 타일들을 논리 배치대로 `out_scale` 해상도의 한 캔버스에 합성한다.
/// 각 모니터 이미지를 (논리크기 × out_scale)로 리사이즈해 배치하므로 DPI가 달라도 배치가 맞다.
pub fn compose(tiles: Vec<Tile>, out_scale: f32) -> RgbaImage {
    if tiles.is_empty() {
        return RgbaImage::new(1, 1);
    }
    let s = if out_scale <= 0.0 { 1.0 } else { out_scale };
    let min_x = tiles.iter().map(|t| t.lx).min().unwrap();
    let min_y = tiles.iter().map(|t| t.ly).min().unwrap();
    let max_x = tiles.iter().map(|t| t.lx + t.lw as i32).max().unwrap();
    let max_y = tiles.iter().map(|t| t.ly + t.lh as i32).max().unwrap();

    let canvas_w = (((max_x - min_x) as f32) * s).round().max(1.0) as u32;
    let canvas_h = (((max_y - min_y) as f32) * s).round().max(1.0) as u32;
    // 모니터가 안 닿는 빈 영역은 투명 대신 불투명 검정으로 채워 흰 줄이 생기지 않게 한다.
    let mut canvas = RgbaImage::from_pixel(canvas_w, canvas_h, image::Rgba([0, 0, 0, 255]));

    for t in tiles {
        let tw = ((t.lw as f32) * s).round().max(1.0) as u32;
        let th = ((t.lh as f32) * s).round().max(1.0) as u32;
        let resized = if t.img.width() == tw && t.img.height() == th {
            t.img
        } else {
            imageops::resize(&t.img, tw, th, imageops::FilterType::Triangle)
        };
        let ox = (((t.lx - min_x) as f32) * s).round() as i64;
        let oy = (((t.ly - min_y) as f32) * s).round() as i64;
        if ox >= 0
            && oy >= 0
            && ox as u32 + tw <= canvas_w
            && oy as u32 + th <= canvas_h
        {
            let _ = canvas.copy_from(&resized, ox as u32, oy as u32);
        }
    }
    canvas
}

/// 모든 모니터를 한 장으로 합성한다(가장 큰 scale 기준 해상도).
pub fn capture_all_displays() -> Result<RgbaImage, String> {
    let monitors = all_monitors()?;
    if monitors.len() == 1 {
        return monitors[0].capture_image().map_err(|e| e.to_string());
    }
    let mut out_scale = 1.0_f32;
    let mut tiles = Vec::new();
    for m in &monitors {
        out_scale = out_scale.max(m.scale_factor().unwrap_or(1.0));
        tiles.push(Tile {
            lx: m.x().map_err(|e| e.to_string())?,
            ly: m.y().map_err(|e| e.to_string())?,
            lw: m.width().map_err(|e| e.to_string())?,
            lh: m.height().map_err(|e| e.to_string())?,
            img: m.capture_image().map_err(|e| e.to_string())?,
        });
    }
    Ok(compose(tiles, out_scale))
}

// ───────────────────── 영역 캡처 ─────────────────────

/// 이미지를 경계 보정하여 사각형으로 크롭한다(순수 로직).
pub fn crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> Result<RgbaImage, String> {
    if w == 0 || h == 0 {
        return Err("선택 영역이 비어 있습니다".into());
    }
    let cx = x.min(img.width().saturating_sub(1));
    let cy = y.min(img.height().saturating_sub(1));
    let cw = w.min(img.width() - cx);
    let ch = h.min(img.height() - cy);
    if cw == 0 || ch == 0 {
        return Err("선택 영역이 화면을 벗어났습니다".into());
    }
    Ok(imageops::crop_imm(img, cx, cy, cw, ch).to_image())
}

/// 전역 논리 좌표(전체 데스크탑 기준)의 사각형을 캡처한다.
/// 선택 중심이 속한 모니터를 찾아 그 모니터를 캡처한 뒤 물리 픽셀로 크롭한다.
pub fn capture_region_global(gx: i32, gy: i32, gw: u32, gh: u32) -> Result<RgbaImage, String> {
    if gw == 0 || gh == 0 {
        return Err("선택 영역이 비어 있습니다".into());
    }
    let cx = gx + gw as i32 / 2;
    let cy = gy + gh as i32 / 2;

    // 중심점을 포함하는 모니터 찾기(논리 좌표 기준).
    let monitors = all_monitors()?;
    let mut chosen: Option<Monitor> = None;
    for m in &monitors {
        let mx = m.x().map_err(|e| e.to_string())?;
        let my = m.y().map_err(|e| e.to_string())?;
        let mw = m.width().map_err(|e| e.to_string())? as i32;
        let mh = m.height().map_err(|e| e.to_string())? as i32;
        if cx >= mx && cx < mx + mw && cy >= my && cy < my + mh {
            chosen = Some(m.clone());
            break;
        }
    }
    let monitor = chosen.ok_or("선택 영역이 화면 밖입니다")?;
    let scale = monitor.scale_factor().unwrap_or(1.0);
    let mx = monitor.x().map_err(|e| e.to_string())?;
    let my = monitor.y().map_err(|e| e.to_string())?;

    // 모니터 로컬 논리 좌표 → 물리 픽셀.
    let local_x = ((gx - mx).max(0) as f32 * scale).round() as u32;
    let local_y = ((gy - my).max(0) as f32 * scale).round() as u32;
    let pw = (gw as f32 * scale).round() as u32;
    let ph = (gh as f32 * scale).round() as u32;

    let full = monitor.capture_image().map_err(|e| e.to_string())?;
    crop(&full, local_x, local_y, pw, ph)
}

// ───────────────────── 창 캡처 ─────────────────────

/// 캡처 대상이 아닌 시스템 UI(제어 센터, Dock 등)는 목록에서 제외.
fn is_system_window(app_name: &str) -> bool {
    const SYSTEM_APPS: [&str; 12] = [
        "Control Center",
        "제어 센터",
        "Notification Center",
        "알림 센터",
        "Dock",
        "WindowManager",
        "Window Server",
        "SystemUIServer",
        "Spotlight",
        "Wallpaper",
        "Screenshot",
        "스크린샷",
    ];
    SYSTEM_APPS.iter().any(|s| s.eq_ignore_ascii_case(app_name))
}

pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
    let windows = Window::all().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for w in windows {
        if w.is_minimized().unwrap_or(false) {
            continue;
        }
        let width = w.width().unwrap_or(0);
        let height = w.height().unwrap_or(0);
        // 너무 작은 창(메뉴바 아이템 등)과 시스템 UI는 제외.
        if width < 80 || height < 80 {
            continue;
        }
        let title = w.title().unwrap_or_default();
        let app_name = w.app_name().unwrap_or_default();
        if app_name.trim().is_empty() || is_system_window(&app_name) {
            continue;
        }
        if let Ok(id) = w.id() {
            out.push(WindowInfo {
                id,
                title,
                app_name,
            });
        }
    }
    Ok(out)
}

/// 호버 강조용 창 사각형 목록(전역 논리 좌표, 앞쪽 창이 먼저).
pub fn list_window_rects() -> Result<Vec<WindowRect>, String> {
    let windows = Window::all().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for w in windows {
        if w.is_minimized().unwrap_or(false) {
            continue;
        }
        let width = w.width().unwrap_or(0);
        let height = w.height().unwrap_or(0);
        if width < 80 || height < 80 {
            continue;
        }
        let app_name = w.app_name().unwrap_or_default();
        if app_name.trim().is_empty() || is_system_window(&app_name) {
            continue;
        }
        // 자기 자신(ClipShot/오버레이)은 제외.
        if app_name.eq_ignore_ascii_case("ClipShot") || app_name.eq_ignore_ascii_case("clip-shot") {
            continue;
        }
        let (Ok(id), Ok(x), Ok(y)) = (w.id(), w.x(), w.y()) else {
            continue;
        };
        out.push(WindowRect {
            id,
            x,
            y,
            width,
            height,
            z: w.z().unwrap_or(0),
            app_name,
        });
    }
    out.sort_by_key(|w| std::cmp::Reverse(w.z)); // 앞쪽(z 큰) 먼저
    Ok(out)
}

pub fn capture_window(id: u32) -> Result<RgbaImage, String> {
    let windows = Window::all().map_err(|e| e.to_string())?;
    for w in windows {
        if w.id().map(|wid| wid == id).unwrap_or(false) {
            return w.capture_image().map_err(|e| e.to_string());
        }
    }
    Err("창을 찾을 수 없습니다(닫혔을 수 있음)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, v: u8) -> RgbaImage {
        RgbaImage::from_pixel(w, h, image::Rgba([v, v, v, 255]))
    }

    #[test]
    fn crop_within_bounds() {
        let img = solid(100, 80, 200);
        assert_eq!(crop(&img, 10, 20, 30, 40).unwrap().dimensions(), (30, 40));
    }

    #[test]
    fn crop_clamps_to_image_edges() {
        let img = solid(100, 80, 200);
        assert_eq!(crop(&img, 90, 70, 50, 50).unwrap().dimensions(), (10, 10));
    }

    #[test]
    fn crop_rejects_empty() {
        let img = solid(10, 10, 0);
        assert!(crop(&img, 0, 0, 0, 5).is_err());
    }

    #[test]
    fn compose_places_side_by_side_same_scale() {
        // 두 모니터(논리 10x10, scale 2 → 물리 20x20)를 가로로 배치, out_scale=2.
        let a = Tile { lx: 0, ly: 0, lw: 10, lh: 10, img: solid(20, 20, 10) };
        let b = Tile { lx: 10, ly: 0, lw: 10, lh: 10, img: solid(20, 20, 20) };
        let canvas = compose(vec![a, b], 2.0);
        assert_eq!(canvas.dimensions(), (40, 20));
        assert_eq!(canvas.get_pixel(5, 5)[0], 10);
        assert_eq!(canvas.get_pixel(25, 5)[0], 20);
    }

    #[test]
    fn compose_handles_offset_and_negative_origin() {
        // 실제 이 Mac 배치와 유사: 주(0,0,1920x1080) + 보조(238,1080,1440x900), scale 2.
        let primary = Tile { lx: 0, ly: 0, lw: 1920, lh: 1080, img: solid(3840, 2160, 1) };
        let second = Tile { lx: 238, ly: 1080, lw: 1440, lh: 900, img: solid(2880, 1800, 2) };
        let canvas = compose(vec![primary, second], 2.0);
        // 합집합 논리: x 0..1920, y 0..1980 → 물리 3840 x 3960
        assert_eq!(canvas.dimensions(), (3840, 3960));
        assert_eq!(canvas.get_pixel(10, 10)[0], 1); // 주 모니터
        assert_eq!(canvas.get_pixel(600, 2200)[0], 2); // 보조 모니터 영역
    }

    #[test]
    fn compose_mixed_scale_keeps_layout() {
        // scale이 다른 모니터도 배치가 유지되어야 한다(out_scale=2 기준 리사이즈).
        let a = Tile { lx: 0, ly: 0, lw: 100, lh: 100, img: solid(200, 200, 50) };
        let b = Tile { lx: 100, ly: 0, lw: 100, lh: 100, img: solid(100, 100, 90) };
        let canvas = compose(vec![a, b], 2.0);
        assert_eq!(canvas.dimensions(), (400, 200));
        assert_eq!(canvas.get_pixel(10, 10)[0], 50);
        assert_eq!(canvas.get_pixel(300, 10)[0], 90); // b가 200..400에 리사이즈되어 배치
    }
}

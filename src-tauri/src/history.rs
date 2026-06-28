//! 캡처 히스토리 저장소.
//!
//! 캡처 1건 = 풀 PNG(`<id>.png`) + 썸네일(`<id>.thumb.png`) + 인덱스 항목.
//! 인덱스는 newest-first. `limit`(기본 30) 초과 시 가장 오래된 항목을 파일째 제거한다(ring buffer).
//!
//! 링버퍼/내보내기 파일명 로직은 파일시스템과 분리한 순수 함수로 두어 단위 테스트한다.

use crate::settings::ImageFormat;
use image::{imageops, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const THUMB_MAX: u32 = 360;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at: u64, // epoch millis
    pub mode: String,    // "region" | "window" | "display" | "all"
    pub width: u32,
    pub height: u32,
}

// ───────────────────────── 순수 로직 (테스트 대상) ─────────────────────────

/// newest-first 인덱스 앞에 새 항목을 넣고, limit를 넘는 가장 오래된 항목들을 잘라
/// 제거 대상으로 돌려준다. `entries`는 limit 이하로 유지된다.
pub fn push_and_trim(
    entries: &mut Vec<HistoryEntry>,
    new_entry: HistoryEntry,
    limit: usize,
) -> Vec<HistoryEntry> {
    entries.insert(0, new_entry);
    if limit == 0 {
        return std::mem::take(entries);
    }
    if entries.len() > limit {
        entries.split_off(limit)
    } else {
        Vec::new()
    }
}

/// 기존 파일명 집합에서 충돌하지 않는 이름을 만든다. 충돌 시 ` (2)`, ` (3)` … 접미사.
/// `stem`은 확장자 없는 기본 이름, `ext`는 확장자(점 제외).
/// 일괄 내보내기에서 사용 예정. 현재는 단위 테스트로 동작을 보증한다.
#[allow(dead_code)]
pub fn unique_filename(stem: &str, ext: &str, existing: &[String]) -> String {
    let set: std::collections::HashSet<&str> = existing.iter().map(|s| s.as_str()).collect();
    let first = format!("{stem}.{ext}");
    if !set.contains(first.as_str()) {
        return first;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{stem} ({n}).{ext}");
        if !set.contains(candidate.as_str()) {
            return candidate;
        }
        n += 1;
    }
}

// ───────────────────────── 파일시스템 저장소 ─────────────────────────

pub fn history_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("history")
}

pub fn index_path(data_dir: &Path) -> PathBuf {
    history_dir(data_dir).join("index.json")
}

fn full_path(data_dir: &Path, id: &str) -> PathBuf {
    history_dir(data_dir).join(format!("{id}.png"))
}

fn thumb_path(data_dir: &Path, id: &str) -> PathBuf {
    history_dir(data_dir).join(format!("{id}.thumb.png"))
}

pub fn load_index(data_dir: &Path) -> Vec<HistoryEntry> {
    match std::fs::read_to_string(index_path(data_dir)) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_index(data_dir: &Path, entries: &[HistoryEntry]) -> Result<(), String> {
    std::fs::create_dir_all(history_dir(data_dir)).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    std::fs::write(index_path(data_dir), text).map_err(|e| e.to_string())
}

fn remove_files(data_dir: &Path, id: &str) {
    let _ = std::fs::remove_file(full_path(data_dir, id));
    let _ = std::fs::remove_file(thumb_path(data_dir, id));
}

/// 캡처 이미지를 히스토리에 추가한다. 풀 PNG + 썸네일 저장, 인덱스 갱신, 초과분 제거.
/// 추가된 항목을 반환한다.
pub fn add(
    data_dir: &Path,
    img: &RgbaImage,
    mode: &str,
    created_at: u64,
    limit: usize,
) -> Result<HistoryEntry, String> {
    std::fs::create_dir_all(history_dir(data_dir)).map_err(|e| e.to_string())?;

    let id = format!("{created_at}");
    let entry = HistoryEntry {
        id: id.clone(),
        created_at,
        mode: mode.to_string(),
        width: img.width(),
        height: img.height(),
    };

    img.save(full_path(data_dir, &id))
        .map_err(|e| e.to_string())?;

    let thumb = imageops::thumbnail(
        img,
        img.width().clamp(1, THUMB_MAX),
        ((img.height() as f64) * (THUMB_MAX as f64) / (img.width().max(1) as f64))
            .round()
            .max(1.0) as u32,
    );
    thumb
        .save(thumb_path(data_dir, &id))
        .map_err(|e| e.to_string())?;

    let mut index = load_index(data_dir);
    let removed = push_and_trim(&mut index, entry.clone(), limit);
    for r in removed {
        remove_files(data_dir, &r.id);
    }
    save_index(data_dir, &index)?;
    Ok(entry)
}

pub fn full_image_path(data_dir: &Path, id: &str) -> Option<PathBuf> {
    let p = full_path(data_dir, id);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

pub fn delete(data_dir: &Path, id: &str) -> Result<(), String> {
    let mut index = load_index(data_dir);
    index.retain(|e| e.id != id);
    remove_files(data_dir, id);
    save_index(data_dir, &index)
}

pub fn clear(data_dir: &Path) -> Result<(), String> {
    for e in load_index(data_dir) {
        remove_files(data_dir, &e.id);
    }
    save_index(data_dir, &[])
}

/// 풀 PNG를 지정 포맷으로 변환해 dest에 저장(내보내기).
pub fn export(data_dir: &Path, id: &str, dest: &Path, format: ImageFormat) -> Result<(), String> {
    let src = full_image_path(data_dir, id).ok_or("원본을 찾을 수 없습니다")?;
    let img = image::open(&src).map_err(|e| e.to_string())?;
    match format {
        ImageFormat::Png => img.to_rgba8().save(dest).map_err(|e| e.to_string()),
        ImageFormat::Jpg => img.to_rgb8().save(dest).map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, ts: u64) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            created_at: ts,
            mode: "display".into(),
            width: 10,
            height: 10,
        }
    }

    #[test]
    fn push_keeps_newest_first() {
        let mut v = vec![entry("a", 1)];
        let removed = push_and_trim(&mut v, entry("b", 2), 30);
        assert!(removed.is_empty());
        assert_eq!(v[0].id, "b");
        assert_eq!(v[1].id, "a");
    }

    #[test]
    fn trim_evicts_oldest_beyond_limit() {
        // newest-first: e0이 맨 앞(최신), e29가 맨 뒤(가장 오래됨).
        let mut v: Vec<HistoryEntry> = (0..30).map(|i| entry(&format!("e{i}"), i)).collect();
        // 현재 30개. 31번째를 넣으면 가장 오래된 1개(e29)가 제거되어야 한다.
        let removed = push_and_trim(&mut v, entry("new", 999), 30);
        assert_eq!(v.len(), 30);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].id, "e29"); // 가장 오래된 항목(맨 뒤)
        assert_eq!(v[0].id, "new");
    }

    #[test]
    fn trim_evicts_multiple_when_far_over() {
        let mut v: Vec<HistoryEntry> = (0..35).map(|i| entry(&format!("e{i}"), i)).collect();
        let removed = push_and_trim(&mut v, entry("new", 999), 30);
        assert_eq!(v.len(), 30);
        // 35 + 1 = 36 → 6개 제거
        assert_eq!(removed.len(), 6);
    }

    #[test]
    fn limit_one_keeps_only_newest() {
        let mut v = vec![entry("a", 1)];
        let removed = push_and_trim(&mut v, entry("b", 2), 1);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "b");
        assert_eq!(removed[0].id, "a");
    }

    #[test]
    fn unique_filename_no_conflict() {
        assert_eq!(unique_filename("shot", "png", &[]), "shot.png");
    }

    #[test]
    fn unique_filename_adds_suffix_on_conflict() {
        let existing = vec!["shot.png".to_string()];
        assert_eq!(unique_filename("shot", "png", &existing), "shot (2).png");
    }

    #[test]
    fn unique_filename_finds_next_free_number() {
        let existing = vec![
            "shot.png".to_string(),
            "shot (2).png".to_string(),
            "shot (3).png".to_string(),
        ];
        assert_eq!(unique_filename("shot", "png", &existing), "shot (4).png");
    }

    #[test]
    fn add_and_evict_on_disk() {
        let dir = std::env::temp_dir().join(format!("clipshot-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let img = RgbaImage::new(8, 8);
        for i in 0..32u64 {
            add(&dir, &img, "display", 1000 + i, 30).unwrap();
        }
        let index = load_index(&dir);
        assert_eq!(index.len(), 30);
        // 가장 오래된 2개(id 1000, 1001) 파일은 사라져야 한다.
        assert!(full_image_path(&dir, "1000").is_none());
        assert!(full_image_path(&dir, "1001").is_none());
        assert!(full_image_path(&dir, "1031").is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

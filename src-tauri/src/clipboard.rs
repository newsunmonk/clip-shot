//! 캡처 이미지를 클립보드에 이미지로 write. 어디서든 즉시 붙여넣기되게 한다.

use image::RgbaImage;
use tauri::image::Image;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

pub fn write_image(app: &AppHandle, img: &RgbaImage) -> Result<(), String> {
    let tauri_img = Image::new_owned(img.as_raw().clone(), img.width(), img.height());
    app.clipboard()
        .write_image(&tauri_img)
        .map_err(|e| e.to_string())
}

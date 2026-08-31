//! 屏幕捕获：基于 `screenshots` crate 抓屏，用 `image` 裁剪。
//!
//! Screen capture: grabs via the `screenshots` crate and crops with `image`.
//!
//! 坐标约定：`capture_rect` 接受全局物理像素（显示器坐标系的绝对值）。
//! `screenshots` 0.8 依赖 image 0.24，与我们用的 image 0.25 类型不互通，
//! 因此经原始 RGBA 缓冲桥接。
//! Coordinate contract: `capture_rect` takes global physical pixels. Note
//! `screenshots` 0.8 depends on image 0.24 while we use 0.25, so frames are
//! bridged through the raw RGBA buffer.

use crate::error::CaptureError;
use image::RgbaImage;

/// 抓取全局物理坐标矩形区域。
/// Captures the rectangle at global physical coordinates.
pub fn capture_rect(x: i32, y: i32, w: u32, h: u32) -> Result<RgbaImage, CaptureError> {
    if w == 0 || h == 0 {
        return Err(CaptureError::Capture("选区宽高为 0".to_string()));
    }
    let monitors = screenshots::Screen::all()
        .map_err(|e| CaptureError::Capture(format!("枚举显示器失败：{e}")))?;
    let monitor = find_monitor(&monitors, x, y)
        .ok_or_else(|| CaptureError::Capture(format!("坐标 ({x},{y}) 不在任何显示器内")))?;
    let info = &monitor.display_info;

    let full = monitor
        .capture()
        .map_err(|e| CaptureError::Capture(format!("屏幕捕获失败：{e}")))?;

    // 桥接 image 0.24 → 0.25：原始 RGBA 缓冲 + 尺寸重建
    // bridge image 0.24 → 0.25 via the raw RGBA buffer
    let (full_w, full_h) = (full.width(), full.height());
    let raw = full.into_raw();
    let full = RgbaImage::from_raw(full_w, full_h, raw)
        .ok_or_else(|| CaptureError::Capture("截图缓冲尺寸不匹配".to_string()))?;

    // 显示器原点 → 全局坐标换算后裁剪；越界部分夹取
    // crop after translating monitor origin into global coords; clamp overscan
    let (mx, my) = (info.x, info.y);
    let cap_w = info.width.min(full_w);
    let cap_h = info.height.min(full_h);
    let rel_x = (x - mx).max(0) as u32;
    let rel_y = (y - my).max(0) as u32;
    let crop_w = w.min(cap_w.saturating_sub(rel_x));
    let crop_h = h.min(cap_h.saturating_sub(rel_y));
    if crop_w == 0 || crop_h == 0 {
        return Err(CaptureError::Capture("选区完全在屏幕之外".to_string()));
    }

    let cropped = image::imageops::crop_imm(&full, rel_x, rel_y, crop_w, crop_h);
    Ok(cropped.to_image())
}

/// 找到包含该点的显示器。
/// Finds the monitor containing the point.
fn find_monitor(monitors: &[screenshots::Screen], x: i32, y: i32) -> Option<screenshots::Screen> {
    monitors
        .iter()
        .find(|m| {
            let info = &m.display_info;
            x >= info.x
                && x < info.x + info.width as i32
                && y >= info.y
                && y < info.y + info.height as i32
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rect_is_rejected() {
        assert!(capture_rect(0, 0, 0, 100).is_err());
        assert!(capture_rect(0, 0, 100, 0).is_err());
    }
}

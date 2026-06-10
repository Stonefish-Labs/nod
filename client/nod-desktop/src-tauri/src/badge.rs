//! Taskbar overlay badge for Windows. Tauri's `set_badge_count` is
//! unsupported there, so the pending count is rasterized onto a small
//! overlay icon for `set_overlay_icon` instead.

use tauri::image::Image;

const SIZE: usize = 32;
const BADGE: [u8; 4] = [214, 48, 49, 255];
const TEXT: [u8; 4] = [255, 255, 255, 255];

pub(crate) fn pending_overlay(count: usize) -> Image<'static> {
    Image::new_owned(render(&label_for(count)), SIZE as u32, SIZE as u32)
}

fn label_for(count: usize) -> String {
    if count > 9 {
        "9+".to_string()
    } else {
        count.to_string()
    }
}

fn render(label: &str) -> Vec<u8> {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let center = (SIZE as f32 - 1.0) / 2.0;
    let radius = SIZE as f32 / 2.0 - 1.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx * dx + dy * dy <= radius * radius {
                put_pixel(&mut rgba, x, y, BADGE);
            }
        }
    }
    draw_label(&mut rgba, label);
    rgba
}

fn draw_label(rgba: &mut [u8], label: &str) {
    let glyph_count = label.chars().count();
    // One digit renders large; "9+" drops a size to fit the circle.
    let scale = if glyph_count == 1 { 4 } else { 3 };
    let gap = scale;
    let label_width = glyph_count * 3 * scale + (glyph_count - 1) * gap;
    let mut x0 = (SIZE - label_width) / 2;
    let y0 = (SIZE - 5 * scale) / 2;
    for ch in label.chars() {
        draw_glyph(rgba, ch, x0, y0, scale);
        x0 += 3 * scale + gap;
    }
}

fn draw_glyph(rgba: &mut [u8], ch: char, x0: usize, y0: usize, scale: usize) {
    let rows = glyph(ch);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..3 {
            if bits & (0b100 >> col) == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    put_pixel(rgba, x0 + col * scale + dx, y0 + row * scale + dy, TEXT);
                }
            }
        }
    }
}

fn put_pixel(rgba: &mut [u8], x: usize, y: usize, color: [u8; 4]) {
    let offset = (y * SIZE + x) * 4;
    rgba[offset..offset + 4].copy_from_slice(&color);
}

fn glyph(ch: char) -> [u8; 5] {
    match ch {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        _ => [0b000, 0b010, 0b111, 0b010, 0b000],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
        let offset = (y * SIZE + x) * 4;
        rgba[offset..offset + 4].try_into().unwrap()
    }

    #[test]
    fn counts_past_nine_collapse_to_nine_plus() {
        assert_eq!(label_for(1), "1");
        assert_eq!(label_for(9), "9");
        assert_eq!(label_for(10), "9+");
        assert_eq!(label_for(140), "9+");
    }

    #[test]
    fn overlay_is_a_circle_with_transparent_corners() {
        let rgba = render("3");

        assert_eq!(pixel(&rgba, 0, 0), [0, 0, 0, 0]);
        assert_eq!(pixel(&rgba, SIZE - 1, SIZE - 1), [0, 0, 0, 0]);
        assert_eq!(pixel(&rgba, SIZE / 2, 2), BADGE);
    }

    #[test]
    fn label_text_lands_white_inside_the_circle() {
        let rgba = render("8");

        let white_pixels = (0..SIZE * SIZE)
            .filter(|index| pixel(&rgba, index % SIZE, index / SIZE) == TEXT)
            .count();
        assert!(white_pixels > 0);
    }

    #[test]
    fn overlay_image_matches_icon_dimensions() {
        let image = pending_overlay(12);

        assert_eq!(image.width(), SIZE as u32);
        assert_eq!(image.height(), SIZE as u32);
    }
}

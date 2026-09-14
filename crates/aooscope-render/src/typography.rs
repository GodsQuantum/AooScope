use fontdue::{Font, FontSettings};
use image::{Rgb, RgbImage};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HorizontalAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub pixel_size: f32,
    pub color: Rgb<u8>,
    pub horizontal_align: HorizontalAlign,
    pub vertical_align: VerticalAlign,
    pub max_lines: Option<usize>,
    pub ellipsis: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            pixel_size: 16.0,
            color: Rgb([53, 217, 255]),
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            max_lines: None,
            ellipsis: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
}

fn font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| {
        Font::from_bytes(
            damascene_fonts_inter::INTER_VARIABLE,
            FontSettings::default(),
        )
        .expect("bundled Inter font must be valid")
    })
}

fn line_height(size: f32) -> f32 {
    (size * 1.2).ceil().max(1.0)
}

fn width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|character| font().metrics(character, size).advance_width)
        .sum()
}

pub fn measure_text(text: &str, style: TextStyle) -> TextMetrics {
    let size = style.pixel_size.max(1.0);
    let lines = text
        .split('\n')
        .take(style.max_lines.unwrap_or(usize::MAX))
        .collect::<Vec<_>>();
    TextMetrics {
        width: lines
            .iter()
            .map(|line| width(line, size))
            .fold(0.0, f32::max),
        height: line_height(size) * lines.len() as f32,
    }
}

fn fit_line(text: &str, max_width: u32, size: f32, ellipsis: bool) -> String {
    if width(text, size) <= max_width as f32 {
        return text.to_owned();
    }
    let suffix = if ellipsis { "…" } else { "" };
    let suffix_width = width(suffix, size);
    let mut result = String::new();
    let mut result_width = 0.0;
    for character in text.chars() {
        let character_width = font().metrics(character, size).advance_width;
        if result_width + character_width + suffix_width > max_width as f32 {
            break;
        }
        result.push(character);
        result_width += character_width;
    }
    if ellipsis {
        result.push('…');
        while width(&result, size) > max_width as f32 && !result.is_empty() {
            result.remove(result.len() - '…'.len_utf8());
            if width(&result, size) + suffix_width <= max_width as f32 {
                result.push('…');
                break;
            }
        }
    }
    result
}

pub fn draw_text(image: &mut RgbImage, bounds: (i32, i32, u32, u32), text: &str, style: TextStyle) {
    let size = style.pixel_size.max(1.0);
    let line_height = line_height(size);
    let max_lines = style.max_lines.unwrap_or(usize::MAX);
    let mut lines = text
        .split('\n')
        .take(max_lines)
        .map(|line| fit_line(line, bounds.2, size, style.ellipsis))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let total_height = line_height * lines.len() as f32;
    let y = bounds.1
        + match style.vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Center => (bounds.3 as f32 - total_height).max(0.0) / 2.0,
            VerticalAlign::Bottom => (bounds.3 as f32 - total_height).max(0.0),
        } as i32;
    for (line_index, line) in lines.iter().enumerate() {
        let line_width = width(line, size);
        let x = bounds.0
            + match style.horizontal_align {
                HorizontalAlign::Left => 0.0,
                HorizontalAlign::Center => (bounds.2 as f32 - line_width).max(0.0) / 2.0,
                HorizontalAlign::Right => (bounds.2 as f32 - line_width).max(0.0),
            } as i32;
        let baseline = y + (line_index as f32 * line_height + size) as i32;
        let mut cursor = x as f32;
        for character in line.chars() {
            let (metrics, bitmap) = font().rasterize(character, size);
            let glyph_x = cursor as i32 + metrics.xmin;
            let glyph_y = baseline - metrics.height as i32 - metrics.ymin;
            for (index, coverage) in bitmap.iter().enumerate() {
                if *coverage == 0 {
                    continue;
                }
                let px = glyph_x + (index % metrics.width) as i32;
                let py = glyph_y + (index / metrics.width) as i32;
                if px < bounds.0
                    || py < bounds.1
                    || px >= bounds.0 + bounds.2 as i32
                    || py >= bounds.1 + bounds.3 as i32
                    || px < 0
                    || py < 0
                    || px >= image.width() as i32
                    || py >= image.height() as i32
                {
                    continue;
                }
                let alpha = *coverage as u16;
                let pixel = image.get_pixel_mut(px as u32, py as u32);
                for channel in 0..3 {
                    pixel[channel] = ((style.color[channel] as u16 * alpha
                        + pixel[channel] as u16 * (255 - alpha))
                        / 255) as u8;
                }
            }
            cursor += metrics.advance_width;
        }
    }
}

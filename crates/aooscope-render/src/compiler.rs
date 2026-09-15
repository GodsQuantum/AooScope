use crate::typography::{HorizontalAlign, TextStyle, VerticalAlign};
use crate::{MediaStore, geometry};
use aooscope_types::{Layer, Page, PagesDocument, StateDocument};
use image::{Rgb, RgbImage, imageops};
use serde_json::Value;
use thiserror::Error;

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 376;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("media: {0}")]
    Media(#[from] crate::MediaError),
}

#[derive(Debug)]
pub struct CompiledPage {
    pub image: RgbImage,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct CompiledDocument {
    pub pages: Vec<CompiledPage>,
    pub switch_seconds: u32,
    pub order: Vec<usize>,
}

fn color(value: Option<&str>, fallback: Rgb<u8>) -> Rgb<u8> {
    value
        .filter(|s| s.len() == 7 && s.starts_with('#'))
        .and_then(|s| u32::from_str_radix(&s[1..], 16).ok())
        .map(|v| Rgb([(v >> 16) as u8, (v >> 8) as u8, v as u8]))
        .unwrap_or(fallback)
}
fn binding_value(state: &StateDocument, binding: Option<&str>) -> Value {
    let Some(binding) = binding else {
        return Value::Null;
    };
    let key = binding.strip_prefix("aooscope_").unwrap_or(binding);
    let mut parts = key.split('_');
    let group = match parts.next() {
        Some("pve") => state.pve.as_ref(),
        Some("hardware") => state.hardware.as_ref(),
        Some("media") => state.media.as_ref(),
        _ => None,
    };
    let parts = parts.collect::<Vec<_>>();
    let resolved = group
        .and_then(|value| resolve_path(value, &parts))
        .cloned()
        .unwrap_or(Value::Null);
    if !resolved.is_null() {
        return resolved;
    }
    let alias = match key {
        "media_display_title_short" => &["display", "title"][..],
        "media_display_headline" => &["display", "mode"][..],
        _ => return Value::Null,
    };
    state
        .media
        .as_ref()
        .and_then(|value| resolve_path(value, alias))
        .cloned()
        .unwrap_or(Value::Null)
}

fn resolve_path<'a>(value: &'a Value, parts: &[&str]) -> Option<&'a Value> {
    if parts.is_empty() {
        return Some(value);
    }
    match value {
        Value::Object(map) => (1..=parts.len()).rev().find_map(|count| {
            let key = parts[..count].join("_");
            map.get(&key)
                .and_then(|child| resolve_path(child, &parts[count..]))
        }),
        Value::Array(items) => parts[0]
            .parse::<usize>()
            .ok()
            .and_then(|index| items.get(index))
            .and_then(|child| resolve_path(child, &parts[1..])),
        _ => None,
    }
}
fn number(state: &StateDocument, binding: Option<&str>) -> f64 {
    binding_value(state, binding).as_f64().unwrap_or(0.0)
}
fn progress(state: &StateDocument, layer: &Layer) -> f64 {
    let raw = if layer.binding.is_some() {
        number(state, layer.binding.as_deref())
    } else {
        layer
            .extra
            .get("value")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };
    let min = layer
        .extra
        .get("min_value")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let max = layer
        .extra
        .get("max_value")
        .and_then(Value::as_f64)
        .unwrap_or(100.0);
    ((raw - min) / (max - min).max(f64::EPSILON)).clamp(0.0, 1.0)
}
fn value_text(state: &StateDocument, binding: Option<&str>, fallback: &str) -> String {
    match binding_value(state, binding) {
        Value::Number(value) => value.to_string(),
        Value::String(value) => value,
        Value::Array(values) => values
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect::<Vec<_>>()
            .join(" / "),
        _ => fallback.into(),
    }
}

fn format_bytes(value: f64, fallback: &str) -> String {
    if value >= 1_000_000_000_000.0 {
        format!("{:.1} TB", value / 1_000_000_000_000.0)
    } else if value >= 1_000_000_000.0 {
        let gigabytes = value / 1_000_000_000.0;
        if gigabytes >= 100.0 {
            format!("{gigabytes:.0} GB")
        } else {
            format!("{gigabytes:.1} GB")
        }
    } else if value >= 1_000_000.0 {
        let megabytes = value / 1_000_000.0;
        if megabytes >= 100.0 {
            format!("{megabytes:.0} MB")
        } else {
            format!("{megabytes:.1} MB")
        }
    } else if value >= 1_000.0 {
        format!("{:.0} KB", value / 1_000.0)
    } else if value > 0.0 {
        format!("{value:.0} B")
    } else {
        fallback.to_owned()
    }
}

fn layer_value_text(state: &StateDocument, layer: &Layer, fallback: &str) -> String {
    if layer.extra.get("format").and_then(Value::as_str) == Some("bytes") {
        return format_bytes(number(state, layer.binding.as_deref()), fallback);
    }
    if layer.extra.get("format").and_then(Value::as_str) == Some("bytes_per_second") {
        let value = number(state, layer.binding.as_deref());
        return if value >= 1_000_000_000.0 {
            format!("{:.1} GB/S", value / 1_000_000_000.0)
        } else if value >= 1_000_000.0 {
            format!("{:.1} MB/S", value / 1_000_000.0)
        } else if value >= 1_000.0 {
            format!("{:.0} KB/S", value / 1_000.0)
        } else if value > 0.0 {
            format!("{value:.0} B/S")
        } else {
            fallback.to_owned()
        };
    }
    value_text(state, layer.binding.as_deref(), fallback)
}
fn rect(image: &mut RgbImage, x: i32, y: i32, w: u32, h: u32, fill: Rgb<u8>) {
    for yy in y.max(0) as u32..(y.max(0) as u32 + h).min(HEIGHT) {
        for xx in x.max(0) as u32..(x.max(0) as u32 + w).min(WIDTH) {
            image.put_pixel(xx, yy, fill);
        }
    }
}
fn opacity(c: Rgb<u8>, value: f64) -> Rgb<u8> {
    Rgb([
        ((c[0] as f64) * value) as u8,
        ((c[1] as f64) * value) as u8,
        ((c[2] as f64) * value) as u8,
    ])
}

pub fn compile_page(
    page: &Page,
    state: &StateDocument,
    media: &MediaStore,
    brightness: u8,
    phase: f64,
) -> Result<CompiledPage, CompileError> {
    let bg = color(Some(&page.background.color), Rgb([7, 16, 25]));
    let mut image = RgbImage::from_pixel(
        WIDTH,
        HEIGHT,
        opacity(bg, brightness.min(100) as f64 / 100.0),
    );
    let mut warnings = Vec::new();
    let mut layers = page.layers.iter().collect::<Vec<_>>();
    layers.sort_by_key(|l| (l.z, l.id.as_str()));
    for layer in layers {
        let intensity = brightness.min(100) as f64 / 100.0 * layer.opacity;
        let c = opacity(
            color(
                layer.extra.get("color").and_then(Value::as_str),
                Rgb([53, 217, 255]),
            ),
            intensity,
        );
        let value = progress(state, layer);
        match layer.layer_type.as_str() {
            "image" => draw_asset(&mut image, layer, state, media, &mut warnings),
            "animation" => {
                if let Some(id) = layer.extra.get("asset_id").and_then(Value::as_str) {
                    draw_asset_id(&mut image, layer, media, id, &mut warnings);
                }
                let cx = layer.x.max(0) as u32 + layer.width / 2;
                let cy = layer.y.max(0) as u32 + layer.height / 2;
                let a = (phase / 100.0 * std::f64::consts::TAU).cos();
                let b = (phase / 100.0 * std::f64::consts::TAU).sin();
                rect(
                    &mut image,
                    (cx as f64 + a * (layer.width as f64 / 2.5) - 4.0) as i32,
                    (cy as f64 + b * (layer.height as f64 / 2.5) - 4.0) as i32,
                    8,
                    8,
                    c,
                );
            }
            "bar" | "gauge" | "ring" => {
                let track = opacity(
                    color(
                        layer.extra.get("track_color").and_then(Value::as_str),
                        Rgb([29, 38, 50]),
                    ),
                    intensity,
                );
                if layer.layer_type == "bar" {
                    let radius = layer
                        .extra
                        .get("radius")
                        .and_then(Value::as_u64)
                        .unwrap_or((layer.width.min(layer.height) / 2) as u64)
                        as u32;
                    geometry::rounded_rect(
                        &mut image,
                        layer.x,
                        layer.y,
                        layer.width,
                        layer.height,
                        radius,
                        track,
                    );
                    if layer.extra.get("orientation").and_then(Value::as_str) == Some("vertical") {
                        let n = (layer.height as f64 * value) as u32;
                        geometry::rounded_rect(
                            &mut image,
                            layer.x,
                            layer.y + (layer.height - n) as i32,
                            layer.width,
                            n,
                            radius.min(n / 2),
                            c,
                        );
                    } else {
                        let n = (layer.width as f64 * value) as u32;
                        geometry::rounded_rect(
                            &mut image,
                            layer.x,
                            layer.y,
                            n,
                            layer.height,
                            radius.min(n / 2),
                            c,
                        );
                    }
                } else {
                    geometry::arc(
                        &mut image,
                        (layer.x, layer.y, layer.width, layer.height),
                        layer
                            .extra
                            .get("thickness")
                            .and_then(Value::as_u64)
                            .unwrap_or(18) as u32,
                        value,
                        track,
                        c,
                        layer.layer_type == "ring",
                    );
                }
            }
            "badge" => draw_badge(&mut image, layer, state, c, intensity),
            "sparkline" => {
                let series = layer.extra.get("series").and_then(Value::as_array);
                if series.is_none_or(|s| s.len() < 2) {
                    warnings.push(format!("{}: sparkline has no history", layer.id));
                } else {
                    for i in 1..series.unwrap().len() {
                        let x = layer.x
                            + ((layer.width as usize * i / ((series.unwrap().len() - 1).max(1)))
                                as i32);
                        rect(&mut image, x, layer.y + (layer.height / 2) as i32, 2, 2, c);
                    }
                }
            }
            "text" | "value" => {
                let mut text = if layer.layer_type == "text" {
                    layer
                        .extra
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| value_text(state, layer.binding.as_deref(), ""))
                } else {
                    layer_value_text(
                        state,
                        layer,
                        layer
                            .extra
                            .get("fallback")
                            .and_then(Value::as_str)
                            .unwrap_or("--"),
                    )
                };
                if let Some(unit) = layer.extra.get("unit").and_then(Value::as_str) {
                    text.push_str(unit);
                }
                draw_layer_text(&mut image, layer, &text, c);
            }
            other => warnings.push(format!("{}: unsupported layer type {}", layer.id, other)),
        }
    }
    Ok(CompiledPage { image, warnings })
}

fn draw_asset(
    image: &mut RgbImage,
    layer: &Layer,
    state: &StateDocument,
    media: &MediaStore,
    warnings: &mut Vec<String>,
) {
    let bound = binding_value(state, layer.binding.as_deref());
    if let Some(id) = layer
        .extra
        .get("asset_id")
        .and_then(Value::as_str)
        .or_else(|| bound.as_str())
    {
        draw_asset_id(image, layer, media, id, warnings)
    } else if layer.extra.get("optional").and_then(Value::as_bool) != Some(true) {
        warnings.push(format!("{}: media unavailable", layer.id));
    }
}

fn draw_badge(
    image: &mut RgbImage,
    layer: &Layer,
    state: &StateDocument,
    color: Rgb<u8>,
    intensity: f64,
) {
    let radius = layer
        .extra
        .get("radius")
        .and_then(Value::as_u64)
        .unwrap_or(12) as u32;
    let background = opacity(
        color_value(layer, "background_color", Rgb([19, 36, 51])),
        intensity,
    );
    geometry::rounded_rect(
        image,
        layer.x,
        layer.y,
        layer.width,
        layer.height,
        radius,
        background,
    );
    if let Some(border) = layer.extra.get("border_color").and_then(Value::as_str) {
        geometry::rounded_rect(
            image,
            layer.x,
            layer.y,
            layer.width,
            layer.height,
            radius,
            opacity(crate_color(border), intensity),
        );
        if layer.width > 2 && layer.height > 2 {
            geometry::rounded_rect(
                image,
                layer.x + 1,
                layer.y + 1,
                layer.width - 2,
                layer.height - 2,
                radius.saturating_sub(1),
                background,
            );
        }
    }
    let text = layer
        .extra
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            layer_value_text(
                state,
                layer,
                layer
                    .extra
                    .get("fallback")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        });
    if !text.is_empty() {
        draw_layer_text(image, layer, &text, color);
    }
}

fn color_value(layer: &Layer, key: &str, fallback: Rgb<u8>) -> Rgb<u8> {
    color(layer.extra.get(key).and_then(Value::as_str), fallback)
}

fn crate_color(value: &str) -> Rgb<u8> {
    color(Some(value), Rgb([53, 217, 255]))
}
fn draw_asset_id(
    image: &mut RgbImage,
    layer: &Layer,
    media: &MediaStore,
    id: &str,
    warnings: &mut Vec<String>,
) {
    match media
        .resolve(id)
        .and_then(|p| image::open(p).map_err(|e| crate::MediaError::Image(e.to_string())))
    {
        Ok(source) => {
            let source = source.to_rgb8();
            let fitted = imageops::resize(
                &source,
                layer.width,
                layer.height,
                imageops::FilterType::Lanczos3,
            );
            imageops::overlay(image, &fitted, layer.x as i64, layer.y as i64);
        }
        Err(e) => warnings.push(format!("{}: media unavailable: {}", layer.id, e)),
    }
}

pub fn compile_document(
    doc: &PagesDocument,
    state: &StateDocument,
    media: &MediaStore,
    brightness: u8,
    phase: f64,
) -> Result<CompiledDocument, CompileError> {
    let mut pages = Vec::new();
    let mut order = Vec::new();
    let mut durations = Vec::new();
    for id in &doc.carousel {
        if let Some(page) = doc.pages.get(id)
            && page.enabled
        {
            durations.push(page.duration.max(2));
            pages.push(compile_page(page, state, media, brightness, phase)?);
        }
    }
    let gcd = durations
        .iter()
        .copied()
        .reduce(gcd)
        .unwrap_or(8)
        .clamp(2, 120);
    for (i, d) in durations.iter().enumerate() {
        order.extend(std::iter::repeat_n(
            i + 1,
            (*d as f64 / gcd as f64).round().max(1.0) as usize,
        ));
    }
    Ok(CompiledDocument {
        pages,
        switch_seconds: gcd,
        order,
    })
}
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn draw_layer_text(image: &mut RgbImage, layer: &Layer, text: &str, color: Rgb<u8>) {
    let scale = layer
        .extra
        .get("scale")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .unwrap_or_else(|| (layer.height / 8).clamp(1, 8))
        .clamp(1, 12);
    let pixel_size = layer
        .extra
        .get("size")
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(scale as f32 * 8.0);
    let horizontal_align = match layer.extra.get("align").and_then(Value::as_str) {
        Some("center") => HorizontalAlign::Center,
        Some("right") => HorizontalAlign::Right,
        _ => HorizontalAlign::Left,
    };
    let vertical_align = match layer.extra.get("valign").and_then(Value::as_str) {
        Some("center") => VerticalAlign::Center,
        Some("bottom") => VerticalAlign::Bottom,
        _ => VerticalAlign::Top,
    };
    crate::typography::draw_text(
        image,
        (layer.x, layer.y, layer.width, layer.height),
        text,
        TextStyle {
            pixel_size,
            color,
            horizontal_align,
            vertical_align,
            max_lines: Some(
                layer
                    .extra
                    .get("max_lines")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or_else(|| {
                        let line_height = (pixel_size * 1.2).ceil().max(1.0);
                        ((layer.height as f32 / line_height).floor() as usize).max(1)
                    }),
            ),
            ellipsis: layer
                .extra
                .get("ellipsis")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        },
    );
}

#[cfg(test)]
mod formatting_tests {
    use super::format_bytes;

    #[test]
    fn bytes_are_human_readable_for_storage_cards() {
        assert_eq!(format_bytes(4_080_000_000_000.0, "--"), "4.1 TB");
        assert_eq!(format_bytes(6_000_000_000_000.0, "--"), "6.0 TB");
        assert_eq!(format_bytes(512_000_000_000.0, "--"), "512 GB");
        assert_eq!(format_bytes(0.0, "--"), "--");
    }
}

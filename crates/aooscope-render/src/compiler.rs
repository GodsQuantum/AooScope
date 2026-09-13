use crate::MediaStore;
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
    let name = parts.collect::<Vec<_>>().join("_");
    group
        .and_then(|v| v.get(&name))
        .cloned()
        .unwrap_or(Value::Null)
}
fn number(state: &StateDocument, binding: Option<&str>) -> f64 {
    binding_value(state, binding).as_f64().unwrap_or(0.0)
}
fn value_text(state: &StateDocument, binding: Option<&str>, fallback: &str) -> String {
    match binding_value(state, binding) {
        Value::Number(value) => value.to_string(),
        Value::String(value) => value,
        _ => fallback.into(),
    }
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
        let c = opacity(
            color(
                layer.extra.get("color").and_then(Value::as_str),
                Rgb([53, 217, 255]),
            ),
            brightness.min(100) as f64 / 100.0,
        );
        let value = (number(state, layer.binding.as_deref()) / 100.0).clamp(0.0, 1.0);
        match layer.layer_type.as_str() {
            "image" => draw_asset(&mut image, layer, media, &mut warnings),
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
                rect(
                    &mut image,
                    layer.x,
                    layer.y,
                    layer.width,
                    layer.height,
                    Rgb([29, 38, 50]),
                );
                if layer.layer_type == "bar" {
                    if layer.extra.get("orientation").and_then(Value::as_str) == Some("vertical") {
                        let n = (layer.height as f64 * value) as u32;
                        rect(
                            &mut image,
                            layer.x,
                            layer.y + (layer.height - n) as i32,
                            layer.width,
                            n,
                            c,
                        );
                    } else {
                        let n = (layer.width as f64 * value) as u32;
                        rect(&mut image, layer.x, layer.y, n, layer.height, c);
                    }
                } else {
                    let n = (layer.width.min(layer.height) as f64 * value) as u32;
                    rect(
                        &mut image,
                        layer.x + ((layer.width - n) / 2) as i32,
                        layer.y + ((layer.height - n) / 2) as i32,
                        n,
                        n,
                        c,
                    );
                }
            }
            "badge" => rect(
                &mut image,
                layer.x,
                layer.y,
                layer.width,
                layer.height,
                color(
                    layer.extra.get("background_color").and_then(Value::as_str),
                    Rgb([19, 36, 51]),
                ),
            ),
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
                let text = if layer.layer_type == "text" {
                    layer
                        .extra
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| value_text(state, layer.binding.as_deref(), ""))
                } else {
                    value_text(
                        state,
                        layer.binding.as_deref(),
                        layer
                            .extra
                            .get("fallback")
                            .and_then(Value::as_str)
                            .unwrap_or("--"),
                    )
                };
                draw_text(&mut image, layer, &text, c);
            }
            other => warnings.push(format!("{}: unsupported layer type {}", layer.id, other)),
        }
    }
    Ok(CompiledPage { image, warnings })
}

fn draw_asset(image: &mut RgbImage, layer: &Layer, media: &MediaStore, warnings: &mut Vec<String>) {
    if let Some(id) = layer.extra.get("asset_id").and_then(Value::as_str) {
        draw_asset_id(image, layer, media, id, warnings)
    } else {
        warnings.push(format!("{}: media unavailable", layer.id));
    }
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

fn draw_text(image: &mut RgbImage, layer: &Layer, text: &str, color: Rgb<u8>) {
    let scale = (layer.height / 8).clamp(1, 8);
    let max_chars = (layer.width / (6 * scale)).max(1) as usize;
    for (row, line) in text
        .as_bytes()
        .chunks(max_chars)
        .take((layer.height / (8 * scale)) as usize)
        .enumerate()
    {
        for (column, byte) in line.iter().enumerate() {
            for (gy, bits) in glyph(*byte).iter().enumerate() {
                for gx in 0..5 {
                    if bits & (1 << (4 - gx)) != 0 {
                        rect(
                            image,
                            layer.x + ((column * 6 + gx) as u32 * scale) as i32,
                            layer.y + ((row * 8 + gy) as u32 * scale) as i32,
                            scale,
                            scale,
                            color,
                        );
                    }
                }
            }
        }
    }
}

fn glyph(c: u8) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        b'0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        b'1' => [0x04, 0x0c, 0x14, 0x04, 0x04, 0x04, 0x1f],
        b'2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        b'3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        b'4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        b'5' => [0x1f, 0x10, 0x10, 0x1e, 0x01, 0x01, 0x1e],
        b'6' => [0x0e, 0x10, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        b'7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        b'9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x01, 0x0e],
        b'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'B' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        b'C' => [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e],
        b'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        b'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        b'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        b'G' => [0x0e, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0f],
        b'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        b'I' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1f],
        b'J' => [0x07, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0c],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        b'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        b'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        b'Q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        b'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        b'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        b'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1b, 0x11],
        b'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        b'-' => [0, 0, 0, 0x1f, 0, 0, 0],
        b'%' => [0x19, 0x19, 0x02, 0x04, 0x08, 0x13, 0x13],
        b'.' => [0, 0, 0, 0, 0, 0x0c, 0x0c],
        b':' => [0, 0x0c, 0x0c, 0, 0x0c, 0x0c, 0],
        b'/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        b'+' => [0, 0x04, 0x04, 0x1f, 0x04, 0x04, 0],
        b' ' => [0; 7],
        _ => [0x1f, 0x11, 0x15, 0x11, 0x15, 0x11, 0x1f],
    }
}

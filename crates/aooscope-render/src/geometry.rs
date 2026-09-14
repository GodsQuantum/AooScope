use image::{Rgb, RgbImage};

pub(crate) fn rounded_rect(
    image: &mut RgbImage,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: u32,
    color: Rgb<u8>,
) {
    let radius = radius.min(width / 2).min(height / 2) as i32;
    for py in 0..height as i32 {
        for px in 0..width as i32 {
            let dx = if px < radius {
                radius - px
            } else if px >= width as i32 - radius {
                px - (width as i32 - radius - 1)
            } else {
                0
            };
            let dy = if py < radius {
                radius - py
            } else if py >= height as i32 - radius {
                py - (height as i32 - radius - 1)
            } else {
                0
            };
            if dx * dx + dy * dy <= radius * radius {
                put(image, x + px, y + py, color);
            }
        }
    }
}

pub(crate) fn arc(
    image: &mut RgbImage,
    bounds: (i32, i32, u32, u32),
    thickness: u32,
    progress: f64,
    track: Rgb<u8>,
    fill: Rgb<u8>,
    ring: bool,
) {
    let (x, y, width, height) = bounds;
    let size = width.min(height) as f64;
    let radius = (size - thickness as f64) / 2.0;
    let inner = radius - thickness as f64 / 2.0;
    let outer = radius + thickness as f64 / 2.0;
    let cx = x as f64 + width as f64 / 2.0;
    let cy = y as f64 + height as f64 / 2.0;
    let (start, sweep) = if ring { (270.0, 360.0) } else { (155.0, 230.0) };
    let end = start + sweep;
    let active_end = start + sweep * progress.clamp(0.0, 1.0);

    for py in y.max(0)..(y + height as i32).min(image.height() as i32) {
        for px in x.max(0)..(x + width as i32).min(image.width() as i32) {
            let dx = px as f64 + 0.5 - cx;
            let dy = py as f64 + 0.5 - cy;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < inner || distance > outer {
                continue;
            }
            let mut angle = dy.atan2(dx).to_degrees();
            if angle < 0.0 {
                angle += 360.0;
            }
            while angle < start {
                angle += 360.0;
            }
            if angle <= end {
                image.put_pixel(
                    px as u32,
                    py as u32,
                    if angle <= active_end { fill } else { track },
                );
            }
        }
    }

    circle_at_angle(
        image,
        cx,
        cy,
        radius,
        start,
        thickness / 2,
        if progress > 0.0 { fill } else { track },
    );
    circle_at_angle(image, cx, cy, radius, end, thickness / 2, track);
    if progress > 0.0 && progress < 1.0 {
        circle_at_angle(image, cx, cy, radius, active_end, thickness / 2, fill);
    }
}

fn circle_at_angle(
    image: &mut RgbImage,
    cx: f64,
    cy: f64,
    radius: f64,
    angle: f64,
    dot_radius: u32,
    color: Rgb<u8>,
) {
    let radians = angle.to_radians();
    let x = cx + radius * radians.cos();
    let y = cy + radius * radians.sin();
    let r = dot_radius as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                put(image, x.round() as i32 + dx, y.round() as i32 + dy, color);
            }
        }
    }
}

fn put(image: &mut RgbImage, x: i32, y: i32, color: Rgb<u8>) {
    if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
        image.put_pixel(x as u32, y as u32, color);
    }
}

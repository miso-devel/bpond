//! SDF-based procedural shark animation.

use ratatui::style::Color;

fn circle(x: f64, y: f64, cx: f64, cy: f64, r: f64) -> f64 {
    ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r
}

fn ellipse(x: f64, y: f64, cx: f64, cy: f64, rx: f64, ry: f64) -> f64 {
    let dx = (x - cx) / rx;
    let dy = (y - cy) / ry;
    (dx * dx + dy * dy).sqrt() * rx.min(ry) - rx.min(ry)
}

fn smooth_union(a: f64, b: f64, k: f64) -> f64 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

fn subtract(a: f64, b: f64) -> f64 {
    a.max(-b)
}

// ─── Shark ──────────────────────────────────────────────────────────────────

fn shark_bend(x: f64, t: f64) -> f64 {
    let tail_factor = ((x + 0.5) * 0.8).clamp(0.0, 1.0);
    (x * 4.0 + t * 4.0).sin() * 0.08 * tail_factor * tail_factor
}

pub fn shark(x: f64, y: f64, t: f64) -> f64 {
    let (x, y) = (x * 2.0, y * 2.0);
    let y = y - shark_bend(x, t);

    // Body: torpedo
    let body = ellipse(x, y, 0.0, 0.0, 0.50, 0.14);

    // Tail fin: V shape
    let tb = (t * 4.0).sin() * 0.06;
    let tail_top = ellipse(x, y, 0.52, -0.10 + tb, 0.12, 0.04);
    let tail_bot = ellipse(x, y, 0.52, 0.10 + tb, 0.12, 0.04);

    // Dorsal fin
    let dorsal = ellipse(x, y, 0.05, -0.18, 0.08, 0.06);

    // Pectoral fin
    let pb = (t * 3.0).sin() * 0.015;
    let pec = ellipse(x, y, -0.10, 0.14 + pb, 0.10, 0.03);

    let mut d = body;
    d = smooth_union(d, tail_top, 0.04);
    d = smooth_union(d, tail_bot, 0.04);
    d = smooth_union(d, dorsal, 0.03);
    d = smooth_union(d, pec, 0.03);

    // Eye hole
    let blink = (t * 0.25).sin();
    let eh = if blink > 0.96 { 0.003 } else { 0.02 };
    d = subtract(d, ellipse(x, y, -0.30, -0.03, 0.025, eh));

    d * 0.5
}

pub fn shark_features(x: f64, y: f64, t: f64) -> Option<(char, Color)> {
    let (x, y) = (x * 2.0, y * 2.0);
    let y = y - shark_bend(x, t);

    let blink = (t * 0.25).sin();
    if blink > 0.96 {
        if ellipse(x, y, -0.30, -0.03, 0.015, 0.003) < 0.0 {
            return Some(('-', Color::Rgb(20, 30, 50)));
        }
        return None;
    }

    // Pupil
    if circle(x, y, -0.30, -0.03, 0.012) < 0.0 {
        return Some(('@', Color::Rgb(10, 15, 30)));
    }
    // Eye white
    if ellipse(x, y, -0.30, -0.03, 0.022, 0.018) < 0.0 {
        return Some(('O', Color::Rgb(220, 225, 235)));
    }

    // Gill slits
    for i in 0..3 {
        let gx = -0.18 + i as f64 * 0.04;
        if ellipse(x, y, gx, 0.0, 0.003, 0.04) < 0.0 {
            return Some(('|', Color::Rgb(60, 80, 120)));
        }
    }

    None
}

/// Map SDF distance to rendering. Returns (char, fg_color).
/// Shark uses outline-focused rendering: thin lines (-|/\) on the edge,
/// light fill inside, so the silhouette is always clear.
pub fn shark_render(d: f64, t: f64, nx: f64, ny: f64) -> (char, Color) {
    let body_r = 0.45;
    let body_g = 0.55;
    let body_b = 0.72;
    let aura_r = 0.20;
    let aura_g = 0.30;
    let aura_b = 0.55;

    if d < -0.025 {
        // Deep inside — light sparse fill
        let brightness = (-d * 6.0).min(1.0);
        let ch = match (brightness * 6.0) as u32 {
            0 => '·',
            1 => '-',
            2 => '=',
            3 => '*',
            4 => '%',
            _ => '$',
        };
        let v = 0.5 + 0.3 * brightness;
        (
            ch,
            Color::Rgb(
                (body_r * v * 255.0) as u8,
                (body_g * v * 255.0) as u8,
                (body_b * v * 255.0) as u8,
            ),
        )
    } else if d < -0.008 {
        // Near edge inside — solid outline
        (
            '#',
            Color::Rgb(
                (body_r * 0.75 * 255.0) as u8,
                (body_g * 0.75 * 255.0) as u8,
                (body_b * 0.75 * 255.0) as u8,
            ),
        )
    } else if d < 0.0 {
        // Outermost edge — bright crisp line
        (
            '@',
            Color::Rgb(
                (body_r * 0.9 * 255.0) as u8,
                (body_g * 0.9 * 255.0) as u8,
                (body_b * 0.9 * 255.0) as u8,
            ),
        )
    } else if d < 0.02 {
        // Tight glow just outside
        let fade = 1.0 - d / 0.02;
        let shimmer = 0.7 + 0.3 * (t * 3.0 + nx * 5.0 + ny * 3.0).sin();
        let v = fade * shimmer * 0.5;
        (
            '+',
            Color::Rgb(
                (aura_r * v * 255.0) as u8,
                (aura_g * v * 255.0) as u8,
                (aura_b * v * 255.0) as u8,
            ),
        )
    } else if d < 0.06 {
        let fade = 1.0 - (d - 0.02) / 0.04;
        let shimmer = 0.5 + 0.5 * (t * 2.0 + nx * 8.0 + ny * 4.0).cos();
        let v = fade * shimmer * 0.25;
        if v > 0.03 {
            (
                '·',
                Color::Rgb(
                    (aura_r * v * 255.0) as u8,
                    (aura_g * v * 255.0) as u8,
                    (aura_b * v * 255.0) as u8,
                ),
            )
        } else {
            (' ', Color::Rgb(10, 8, 16))
        }
    } else {
        (' ', Color::Rgb(10, 8, 16))
    }
}

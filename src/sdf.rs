//! SDF-based procedural character animation.

use ratatui::style::Color;

fn circle(x: f64, y: f64, cx: f64, cy: f64, r: f64) -> f64 {
    ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r
}

fn ellipse(x: f64, y: f64, cx: f64, cy: f64, rx: f64, ry: f64) -> f64 {
    let dx = (x - cx) / rx;
    let dy = (y - cy) / ry;
    (dx * dx + dy * dy).sqrt() * rx.min(ry) - rx.min(ry)
}

fn rounded_rect(x: f64, y: f64, cx: f64, cy: f64, hw: f64, hh: f64, r: f64) -> f64 {
    let dx = ((x - cx).abs() - hw + r).max(0.0);
    let dy = ((y - cy).abs() - hh + r).max(0.0);
    (dx * dx + dy * dy).sqrt() - r
}

fn smooth_union(a: f64, b: f64, k: f64) -> f64 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

fn subtract(a: f64, b: f64) -> f64 {
    a.max(-b)
}

// ─── Ghost ──────────────────────────────────────────────────────────────────

pub fn ghost(x: f64, y: f64, t: f64) -> f64 {
    let bob = (t * 0.8).sin() * 0.03;
    let y = y - bob;

    let head = circle(x, y, 0.0, -0.15, 0.38);
    let body = rounded_rect(x, y, 0.0, 0.25, 0.34, 0.30, 0.05);
    let mut d = smooth_union(head, body, 0.15);

    let wave1 = (x * 8.0 + t * 2.5).sin() * 0.06;
    let wave2 = (x * 12.0 - t * 1.8).sin() * 0.03;
    let wave3 = (x * 5.0 + t * 1.2).cos() * 0.04;
    d = d.max(-(y - (0.48 + wave1 + wave2 + wave3)));

    let blink = (t * 0.4).sin();
    let eye_h = if blink > 0.95 { 0.015 } else { 0.11 };
    d = subtract(d, ellipse(x, y, -0.15, -0.15, 0.10, eye_h));
    d = subtract(d, ellipse(x, y, 0.15, -0.15, 0.10, eye_h));

    d
}

pub fn ghost_features(x: f64, y: f64, t: f64) -> Option<(char, Color)> {
    let bob = (t * 0.8).sin() * 0.03;
    let y = y - bob;
    let blink = (t * 0.4).sin();

    if blink > 0.95 {
        if ellipse(x, y, -0.15, -0.15, 0.06, 0.008) < 0.0
            || ellipse(x, y, 0.15, -0.15, 0.06, 0.008) < 0.0
        {
            return Some(('=', Color::Rgb(255, 255, 255)));
        }
        return None;
    }

    let px = (t * 0.5).sin() * 0.02;
    let py = (t * 0.3).cos() * 0.015;
    if circle(x, y, -0.15 + px, -0.14 + py, 0.045) < 0.0
        || circle(x, y, 0.15 + px, -0.14 + py, 0.045) < 0.0
    {
        return Some(('@', Color::Rgb(255, 255, 255)));
    }

    None
}

// ─── Shark ──────────────────────────────────────────────────────────────────
// Simple torpedo shape with undulating body motion.
// The entire body bends with a sine wave that travels nose-to-tail,
// creating a natural fish swimming motion.

pub fn shark(x: f64, y: f64, t: f64) -> f64 {
    // === Undulation: bend the coordinate space ===
    // A sine wave displaces y based on x position, traveling from head to tail.
    // Amplitude increases toward the tail (fish physics).
    let tail_factor = ((x + 0.5) * 0.8).clamp(0.0, 1.0); // 0 at nose, 1 at tail
    let bend = (x * 4.0 + t * 4.0).sin() * 0.08 * tail_factor * tail_factor;
    let y = y - bend;

    // === Body: tapered ellipse (torpedo shape) ===
    // Wider at center, narrows toward both ends
    let body = ellipse(x, y, 0.0, 0.0, 0.50, 0.14);

    // === Tail fin: triangle at the back ===
    // Two angled ellipses forming a V
    let tail_bend = (t * 4.0).sin() * 0.06;
    let tail_top = ellipse(x, y, 0.52, -0.10 + tail_bend, 0.12, 0.04);
    let tail_bot = ellipse(x, y, 0.52, 0.10 + tail_bend, 0.12, 0.04);

    // === Dorsal fin: triangle on top ===
    let dorsal = ellipse(x, y, 0.05, -0.18, 0.08, 0.06);

    // === Pectoral fins: small, on sides ===
    let pec_bend = (t * 3.0).sin() * 0.015;
    let pec = ellipse(x, y, -0.10, 0.14 + pec_bend, 0.10, 0.03);

    let mut d = body;
    d = smooth_union(d, tail_top, 0.04);
    d = smooth_union(d, tail_bot, 0.04);
    d = smooth_union(d, dorsal, 0.03);
    d = smooth_union(d, pec, 0.03);

    // === Eye: small hole ===
    let blink = (t * 0.25).sin();
    let eye_h = if blink > 0.96 { 0.003 } else { 0.02 };
    d = subtract(d, ellipse(x, y, -0.30, -0.03, 0.025, eye_h));

    d
}

pub fn shark_features(x: f64, y: f64, t: f64) -> Option<(char, Color)> {
    let tail_factor = ((x + 0.5) * 0.8).clamp(0.0, 1.0);
    let bend = (x * 4.0 + t * 4.0).sin() * 0.08 * tail_factor * tail_factor;
    let y = y - bend;

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

    // Gill slits — 3 small lines
    for i in 0..3 {
        let gx = -0.18 + i as f64 * 0.04;
        if ellipse(x, y, gx, 0.0, 0.003, 0.04) < 0.0 {
            return Some(('|', Color::Rgb(60, 80, 120)));
        }
    }

    None
}

// ─── Shared ─────────────────────────────────────────────────────────────────

pub fn density_char(brightness: f64) -> char {
    match (brightness * 10.0) as u32 {
        0 => '·',
        1 => '+',
        2 => '=',
        3 => '*',
        4 => '%',
        5 => '%',
        6 => '$',
        7 => '$',
        8 => '@',
        9 => '@',
        _ => '@',
    }
}

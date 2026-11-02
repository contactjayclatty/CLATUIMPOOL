//! Tiny immediate-mode UI helpers: panels, buttons, switches, sliders and
//! segmented pickers, all drawn with macroquad primitives.

use macroquad::prelude::*;

pub const BG: Color = Color::new(0.055, 0.065, 0.085, 1.0);
pub const PANEL: Color = Color::new(0.085, 0.100, 0.130, 0.97);
pub const PANEL_SOFT: Color = Color::new(0.125, 0.145, 0.185, 0.97);
pub const EDGE: Color = Color::new(1.0, 1.0, 1.0, 0.09);
pub const TEXT: Color = Color::new(0.90, 0.93, 0.97, 1.0);
pub const DIM: Color = Color::new(0.55, 0.59, 0.66, 1.0);
pub const ACCENT: Color = Color::new(0.42, 0.85, 0.56, 1.0);
pub const GOLD: Color = Color::new(0.96, 0.78, 0.30, 1.0);
pub const DANGER: Color = Color::new(0.90, 0.38, 0.38, 1.0);

pub fn blend(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Rounded rectangle built from two bars and four corner discs.
pub fn rounded(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    let r = radius.min(w * 0.5).min(h * 0.5);
    if r <= 0.5 {
        draw_rectangle(x, y, w, h, color);
        return;
    }
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    draw_rectangle(x, y + r, w, h - 2.0 * r, color);
    for (cx, cy) in [
        (x + r, y + r),
        (x + w - r, y + r),
        (x + r, y + h - r),
        (x + w - r, y + h - r),
    ] {
        draw_circle(cx, cy, r, color);
    }
}

pub fn outline(x: f32, y: f32, w: f32, h: f32, color: Color) {
    draw_rectangle_lines(x, y, w, h, 1.0, color);
}

pub fn centered(text: &str, cx: f32, baseline: f32, size: f32, color: Color) {
    let metrics = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - metrics.width * 0.5, baseline, size, color);
}

pub fn right_aligned(text: &str, right: f32, baseline: f32, size: f32, color: Color) {
    let metrics = measure_text(text, None, size as u16, 1.0);
    draw_text(text, right - metrics.width, baseline, size, color);
}

fn mouse_clicked() -> bool {
    is_mouse_button_pressed(MouseButton::Left)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Normal,
    Danger,
}

/// A button. Returns true on the frame it is clicked.
pub fn button(rect: Rect, label: &str, kind: Kind, size: f32) -> bool {
    let (mx, my) = mouse_position();
    let hover = rect.contains(vec2(mx, my));
    let down = hover && is_mouse_button_down(MouseButton::Left);
    let base = match kind {
        Kind::Primary => Color::new(0.13, 0.36, 0.23, 1.0),
        Kind::Normal => PANEL_SOFT,
        Kind::Danger => Color::new(0.33, 0.13, 0.15, 1.0),
    };
    let fill = if down {
        blend(base, Color::new(0.0, 0.0, 0.0, 1.0), 0.25)
    } else if hover {
        blend(base, Color::new(1.0, 1.0, 1.0, 1.0), 0.10)
    } else {
        base
    };
    rounded(rect.x, rect.y, rect.w, rect.h, 8.0, fill);
    let border = match kind {
        Kind::Primary => ACCENT,
        Kind::Danger => DANGER,
        Kind::Normal => EDGE,
    };
    outline(rect.x, rect.y, rect.w, rect.h, border);
    centered(
        label,
        rect.x + rect.w * 0.5,
        rect.y + rect.h * 0.5 + size * 0.36,
        size,
        TEXT,
    );
    hover && mouse_clicked()
}

/// A settings row with a switch on the right. Returns the new value.
pub fn toggle(rect: Rect, label: &str, value: bool) -> bool {
    let (mx, my) = mouse_position();
    let hover = rect.contains(vec2(mx, my));
    if hover {
        rounded(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            6.0,
            Color::new(1.0, 1.0, 1.0, 0.035),
        );
    }
    draw_text(
        label,
        rect.x + 14.0,
        rect.y + rect.h * 0.5 + 6.0,
        19.0,
        TEXT,
    );

    let w = 46.0;
    let h = 24.0;
    let x = rect.x + rect.w - w - 14.0;
    let y = rect.y + (rect.h - h) * 0.5;
    let pill = if value {
        blend(ACCENT, Color::new(0.0, 0.2, 0.05, 1.0), 0.25)
    } else {
        Color::new(0.24, 0.26, 0.31, 1.0)
    };
    rounded(x, y, w, h, h * 0.5, pill);
    let knob_x = if value { x + w - h * 0.5 } else { x + h * 0.5 };
    draw_circle(
        knob_x,
        y + h * 0.5,
        h * 0.5 - 3.0,
        Color::new(0.97, 0.98, 1.0, 1.0),
    );

    if hover && mouse_clicked() {
        !value
    } else {
        value
    }
}

/// A settings row with a horizontal slider. Returns the new value.
pub fn slider(rect: Rect, label: &str, value: f32, min: f32, max: f32, display: &str) -> f32 {
    let (mx, my) = mouse_position();
    let track = Rect::new(
        rect.x + rect.w * 0.46,
        rect.y + rect.h * 0.5 - 3.0,
        rect.w * 0.36,
        6.0,
    );
    let grab = Rect::new(track.x - 10.0, rect.y, track.w + 20.0, rect.h);
    let hover = grab.contains(vec2(mx, my));
    if hover {
        rounded(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            6.0,
            Color::new(1.0, 1.0, 1.0, 0.035),
        );
    }
    draw_text(
        label,
        rect.x + 14.0,
        rect.y + rect.h * 0.5 + 6.0,
        19.0,
        TEXT,
    );

    rounded(
        track.x,
        track.y,
        track.w,
        track.h,
        3.0,
        Color::new(0.22, 0.24, 0.29, 1.0),
    );
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
    rounded(track.x, track.y, track.w * ratio, track.h, 3.0, ACCENT);
    let knob = track.x + track.w * ratio;
    draw_circle(
        knob,
        track.y + track.h * 0.5,
        8.0,
        Color::new(0.95, 0.97, 1.0, 1.0),
    );

    right_aligned(
        display,
        rect.x + rect.w - 16.0,
        rect.y + rect.h * 0.5 + 6.0,
        17.0,
        DIM,
    );

    if hover && is_mouse_button_down(MouseButton::Left) {
        let t = ((mx - track.x) / track.w).clamp(0.0, 1.0);
        min + (max - min) * t
    } else {
        value
    }
}

/// A settings row with a row of small choice chips. Returns the new index.
pub fn segmented(rect: Rect, label: &str, options: &[&str], selected: usize) -> usize {
    let (mx, my) = mouse_position();
    if rect.contains(vec2(mx, my)) {
        rounded(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            6.0,
            Color::new(1.0, 1.0, 1.0, 0.035),
        );
    }
    draw_text(
        label,
        rect.x + 14.0,
        rect.y + rect.h * 0.5 + 6.0,
        19.0,
        TEXT,
    );

    let chip_w = 92.0;
    let gap = 6.0;
    let total = chip_w * options.len() as f32 + gap * (options.len() as f32 - 1.0);
    let mut x = rect.x + rect.w - total - 14.0;
    let y = rect.y + 8.0;
    let h = rect.h - 16.0;
    let mut picked = selected;
    for (index, option) in options.iter().enumerate() {
        let chip = Rect::new(x, y, chip_w, h);
        let active = index == selected;
        let hover = chip.contains(vec2(mx, my));
        let fill = if active {
            blend(ACCENT, Color::new(0.0, 0.15, 0.05, 1.0), 0.35)
        } else if hover {
            blend(PANEL_SOFT, Color::new(1.0, 1.0, 1.0, 1.0), 0.10)
        } else {
            PANEL_SOFT
        };
        rounded(chip.x, chip.y, chip.w, chip.h, 6.0, fill);
        centered(
            option,
            chip.x + chip.w * 0.5,
            chip.y + chip.h * 0.5 + 5.0,
            15.0,
            if active { TEXT } else { DIM },
        );
        if hover && mouse_clicked() {
            picked = index;
        }
        x += chip_w + gap;
    }
    picked
}

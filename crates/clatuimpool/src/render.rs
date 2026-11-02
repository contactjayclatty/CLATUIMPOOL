use macroquad::prelude::*;
use pool_core::Vec2 as Wv;
use pool_core::{Ball, PLAY_LENGTH, PLAY_WIDTH, Table};

/// Width of the wooden rail around the playing surface, in metres.
pub const RAIL_WIDTH: f64 = 0.115;

/// Part of the rail that is covered by the cushion, as a fraction of [`RAIL_WIDTH`].
const CUSHION_FRACTION: f32 = 0.46;

/// Cloth and frame colours for the table.
pub struct Palette {
    pub felt: Color,
    pub cushion: Color,
    pub wood: Color,
    pub wood_dark: Color,
    pub sight: Color,
}

/// Selectable table finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableTheme {
    Classic,
    Blue,
    Violet,
    Midnight,
}

impl TableTheme {
    pub const ALL: [Self; 4] = [Self::Classic, Self::Blue, Self::Violet, Self::Midnight];

    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Blue => "tournament",
            Self::Violet => "violet",
            Self::Midnight => "midnight",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Classic => 0,
            Self::Blue => 1,
            Self::Violet => 2,
            Self::Midnight => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Classic)
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::Classic => Palette {
                felt: Color::new(0.045, 0.255, 0.155, 1.0),
                cushion: Color::new(0.028, 0.19, 0.115, 1.0),
                wood: Color::new(0.27, 0.135, 0.075, 1.0),
                wood_dark: Color::new(0.135, 0.062, 0.035, 1.0),
                sight: Color::new(0.95, 0.90, 0.70, 1.0),
            },
            Self::Blue => Palette {
                felt: Color::new(0.05, 0.21, 0.44, 1.0),
                cushion: Color::new(0.03, 0.14, 0.31, 1.0),
                wood: Color::new(0.30, 0.17, 0.10, 1.0),
                wood_dark: Color::new(0.16, 0.09, 0.05, 1.0),
                sight: Color::new(0.96, 0.94, 0.86, 1.0),
            },
            Self::Violet => Palette {
                felt: Color::new(0.16, 0.09, 0.28, 1.0),
                cushion: Color::new(0.11, 0.06, 0.20, 1.0),
                wood: Color::new(0.31, 0.18, 0.20, 1.0),
                wood_dark: Color::new(0.17, 0.09, 0.11, 1.0),
                sight: Color::new(0.95, 0.90, 0.85, 1.0),
            },
            Self::Midnight => Palette {
                felt: Color::new(0.085, 0.10, 0.12, 1.0),
                cushion: Color::new(0.05, 0.06, 0.08, 1.0),
                wood: Color::new(0.20, 0.20, 0.23, 1.0),
                wood_dark: Color::new(0.10, 0.10, 0.12, 1.0),
                sight: Color::new(0.90, 0.90, 0.92, 1.0),
            },
        }
    }
}

/// Maps table metres onto screen pixels, keeping the whole table visible.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub scale: f32,
    pub center: Vec2,
}

impl View {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let table_width = (PLAY_LENGTH + 2.0 * RAIL_WIDTH + 0.14) as f32;
        let table_height = (PLAY_WIDTH + 2.0 * RAIL_WIDTH + 0.14) as f32;
        let scale =
            ((screen_width - 40.0) / table_width).min((screen_height - 176.0) / table_height);
        Self {
            scale: scale.max(1.0),
            center: vec2(screen_width * 0.5, screen_height * 0.5 - 2.0),
        }
    }

    /// Table coordinates (metres, y up) to screen pixels (y down).
    pub fn to_screen(self, world: Wv) -> Vec2 {
        vec2(
            self.center.x + world.x as f32 * self.scale,
            self.center.y - world.y as f32 * self.scale,
        )
    }

    pub fn to_world(self, pixel: Vec2) -> Wv {
        Wv::new(
            ((pixel.x - self.center.x) / self.scale) as f64,
            ((self.center.y - pixel.y) / self.scale) as f64,
        )
    }

    /// Screen length for a world-space length.
    pub fn len(self, metres: f64) -> f32 {
        (metres * self.scale as f64) as f32
    }
}

/// Standard pool ball colours. Ball `0` is the white cue ball, `8` is black and
/// `9..=15` use the colour of the matching solid.
pub fn ball_color(id: u8) -> Color {
    let solid = if (9..=15).contains(&id) { id - 8 } else { id };
    match solid {
        0 => Color::new(0.97, 0.96, 0.93, 1.0),
        1 => Color::new(1.00, 0.82, 0.00, 1.0),
        2 => Color::new(0.10, 0.30, 0.66, 1.0),
        3 => Color::new(0.78, 0.06, 0.18, 1.0),
        4 => Color::new(0.36, 0.17, 0.55, 1.0),
        5 => Color::new(0.94, 0.50, 0.00, 1.0),
        6 => Color::new(0.00, 0.52, 0.24, 1.0),
        7 => Color::new(0.48, 0.12, 0.17, 1.0),
        _ => Color::new(0.09, 0.09, 0.10, 1.0),
    }
}

pub fn is_striped(id: u8) -> bool {
    (9..=15).contains(&id)
}

fn fade(color: Color, alpha: f32) -> Color {
    Color::new(color.r, color.g, color.b, color.a * alpha)
}

fn blend(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Rounded rectangle built from two bars and four corner discs.
fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    let r = radius.min(w * 0.5).min(h * 0.5);
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

/// Fills a rectangle with a vertical gradient, trimming the corners so the
/// result still reads as a rounded panel.
fn rounded_gradient(rect: Rect, radius: f32, top: Color, bottom: Color) {
    const STEPS: usize = 56;
    let rad = radius.min(rect.w * 0.5).min(rect.h * 0.5);
    for step in 0..STEPS {
        let t = step as f32 / STEPS as f32;
        let y = rect.y + rect.h * t;
        let height = rect.h / STEPS as f32 + 1.0;
        let inset = |row: f32| -> f32 {
            let dy = (row - rect.y).min(rect.y + rect.h - row);
            if dy >= rad {
                0.0
            } else {
                rad - (rad * rad - dy * dy).max(0.0).sqrt()
            }
        };
        let ins = inset(y).max(inset(y + height));
        draw_rectangle(
            rect.x + ins,
            y,
            rect.w - ins * 2.0,
            height,
            blend(top, bottom, t),
        );
    }
}

/// One pixel outline that follows a rounded rectangle.
fn stroke_rounded(rect: Rect, radius: f32, thickness: f32, color: Color) {
    let r = radius.min(rect.w * 0.5).min(rect.h * 0.5);
    draw_line(
        rect.x + r,
        rect.y,
        rect.x + rect.w - r,
        rect.y,
        thickness,
        color,
    );
    draw_line(
        rect.x + r,
        rect.y + rect.h,
        rect.x + rect.w - r,
        rect.y + rect.h,
        thickness,
        color,
    );
    draw_line(
        rect.x,
        rect.y + r,
        rect.x,
        rect.y + rect.h - r,
        thickness,
        color,
    );
    draw_line(
        rect.x + rect.w,
        rect.y + r,
        rect.x + rect.w,
        rect.y + rect.h - r,
        thickness,
        color,
    );
    draw_arc(rect.x + r, rect.y + r, 8, r, 180.0, thickness, 90.0, color);
    draw_arc(
        rect.x + rect.w - r,
        rect.y + r,
        8,
        r,
        270.0,
        thickness,
        90.0,
        color,
    );
    draw_arc(
        rect.x + r,
        rect.y + rect.h - r,
        8,
        r,
        90.0,
        thickness,
        90.0,
        color,
    );
    draw_arc(
        rect.x + rect.w - r,
        rect.y + rect.h - r,
        8,
        r,
        0.0,
        thickness,
        90.0,
        color,
    );
}

pub fn draw_table(table: &Table, view: &View, theme: TableTheme) {
    let p = theme.palette();
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    let black = Color::new(0.0, 0.0, 0.0, 1.0);
    let center = view.to_screen(Wv::ZERO);
    let s = view.scale;
    let hl = table.half_length as f32 * s;
    let hw = table.half_width as f32 * s;
    let rail = (RAIL_WIDTH * s as f64) as f32;

    let felt = Rect::new(center.x - hl, center.y - hw, hl * 2.0, hw * 2.0);
    let frame = Rect::new(
        felt.x - rail,
        felt.y - rail,
        felt.w + rail * 2.0,
        felt.h + rail * 2.0,
    );

    // Table shadow on the floor, built from a few soft layers.
    rounded_rect(
        frame.x - 10.0,
        frame.y + 20.0,
        frame.w + 20.0,
        frame.h,
        28.0,
        Color::new(0.0, 0.0, 0.0, 0.10),
    );
    rounded_rect(
        frame.x - 4.0,
        frame.y + 12.0,
        frame.w + 8.0,
        frame.h,
        22.0,
        Color::new(0.0, 0.0, 0.0, 0.13),
    );
    rounded_rect(
        frame.x,
        frame.y + 5.0,
        frame.w,
        frame.h,
        20.0,
        Color::new(0.0, 0.0, 0.0, 0.18),
    );

    // Lacquered wood: lit along the top edge, deep at the bottom.
    rounded_gradient(
        frame,
        18.0,
        blend(p.wood, white, 0.22),
        blend(p.wood_dark, black, 0.12),
    );
    stroke_rounded(
        Rect::new(frame.x + 1.5, frame.y + 1.5, frame.w - 3.0, frame.h - 3.0),
        17.0,
        1.5,
        Color::new(1.0, 1.0, 1.0, 0.16),
    );
    stroke_rounded(
        Rect::new(frame.x + 4.0, frame.y + 4.0, frame.w - 8.0, frame.h - 8.0),
        15.0,
        1.5,
        Color::new(0.0, 0.0, 0.0, 0.30),
    );

    // Wood grain: soft horizontal streaks, kept clear of the rounded corners.
    let grain_x = frame.x + 18.0;
    let grain_w = frame.w - 36.0;
    for step in 0..26 {
        let t = step as f32 / 26.0;
        let y = frame.y + 10.0 + t * (frame.h - 20.0);
        let alpha = if step % 3 == 0 { 0.05 } else { 0.03 };
        draw_line(
            grain_x,
            y,
            grain_x + grain_w,
            y,
            1.2,
            Color::new(0.0, 0.0, 0.0, alpha),
        );
    }

    // Cushion cloth sits between the wood and the playing surface.
    let cushion = rail * CUSHION_FRACTION;
    let cushion_color = blend(blend(p.felt, p.cushion, 0.5), white, 0.06);
    draw_rectangle(
        felt.x - cushion,
        felt.y - cushion,
        felt.w + cushion * 2.0,
        felt.h + cushion * 2.0,
        cushion_color,
    );
    // The cushion face is lit, the outer edge falls into the rail.
    let face = cushion * 0.62;
    let face_color = blend(cushion_color, white, 0.06);
    draw_rectangle(
        felt.x - face,
        felt.y - face,
        felt.w + face * 2.0,
        face,
        face_color,
    );
    draw_rectangle(
        felt.x - face,
        felt.y,
        face,
        felt.h,
        blend(cushion_color, black, 0.04),
    );
    draw_rectangle(
        felt.x + felt.w,
        felt.y,
        face,
        felt.h,
        blend(cushion_color, black, 0.18),
    );
    draw_rectangle(
        felt.x - face,
        felt.y + felt.h,
        felt.w + face * 2.0,
        face,
        blend(cushion_color, black, 0.30),
    );
    // Outer edge of the cushion drops into a recess before the wood.
    let recess = 2.5;
    draw_rectangle(
        felt.x - cushion,
        felt.y - cushion,
        felt.w + cushion * 2.0,
        recess,
        Color::new(0.0, 0.0, 0.0, 0.35),
    );
    draw_rectangle(
        felt.x - cushion,
        felt.y - cushion,
        recess,
        felt.h + cushion * 2.0,
        Color::new(0.0, 0.0, 0.0, 0.30),
    );
    draw_rectangle(
        felt.x + felt.w + cushion - recess,
        felt.y - cushion,
        recess,
        felt.h + cushion * 2.0,
        Color::new(0.0, 0.0, 0.0, 0.30),
    );

    // Nose of each cushion catches the light, with a shadow on the cloth below.
    let nose = blend(cushion_color, white, 0.14);
    draw_line(felt.x, felt.y, felt.x + felt.w, felt.y, 2.0, nose);
    draw_line(
        felt.x,
        felt.y,
        felt.x,
        felt.y + felt.h,
        2.0,
        blend(cushion_color, white, 0.08),
    );
    draw_line(
        felt.x + felt.w,
        felt.y,
        felt.x + felt.w,
        felt.y + felt.h,
        1.5,
        Color::new(0.0, 0.0, 0.0, 0.40),
    );
    draw_line(
        felt.x,
        felt.y + felt.h,
        felt.x + felt.w,
        felt.y + felt.h,
        1.5,
        Color::new(0.0, 0.0, 0.0, 0.40),
    );

    // Playing surface, slightly lighter towards the head of the table.
    rounded_gradient(
        felt,
        3.0,
        blend(p.felt, white, 0.07),
        blend(p.felt, black, 0.16),
    );

    // Pool of light above the table, so the cloth is not a flat fill.
    for step in 0..22 {
        let t = step as f32 / 22.0;
        let shrink = 1.0 - t * 0.86;
        draw_ellipse(
            center.x,
            felt.y + felt.h * 0.42,
            felt.w * 0.56 * shrink,
            felt.h * 0.78 * shrink,
            0.0,
            Color::new(1.0, 1.0, 1.0, 0.007),
        );
    }

    // Vignette: the cloth falls away into shadow as it meets the cushions.
    let depth = (rail * 1.45).min(felt.h * 0.18);
    for step in 0..depth as usize {
        let inset = step as f32;
        draw_rectangle_lines(
            felt.x + inset,
            felt.y + inset,
            felt.w - inset * 2.0,
            felt.h - inset * 2.0,
            1.0,
            Color::new(0.0, 0.0, 0.0, 0.014),
        );
    }

    // Cloth grain: a light dusting of fibres so the felt is not a flat fill.
    let mut seed = 0x2f6e_2d3bu32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let specks = ((felt.w * felt.h) / 1400.0) as usize;
    for _ in 0..specks {
        let x = felt.x + (next() % 10_000) as f32 / 10_000.0 * felt.w;
        let y = felt.y + (next() % 10_000) as f32 / 10_000.0 * felt.h;
        let light = next() % 2 == 0;
        let color = if light {
            Color::new(1.0, 1.0, 1.0, 0.022)
        } else {
            Color::new(0.0, 0.0, 0.0, 0.030)
        };
        draw_circle(x, y, 0.9, color);
    }

    // Cushions throw a soft shadow onto the cloth.
    let shade = rail * 0.55;
    for step in 0..10 {
        let t = step as f32 / 10.0;
        let alpha = 0.055 * (1.0 - t);
        let inset = shade * t;
        draw_rectangle(
            felt.x + inset,
            felt.y + inset,
            felt.w - inset * 2.0,
            shade - inset + 2.0,
            Color::new(0.0, 0.0, 0.0, alpha),
        );
        draw_rectangle(
            felt.x + inset,
            felt.y + inset,
            shade - inset + 2.0,
            felt.h - inset * 2.0,
            Color::new(0.0, 0.0, 0.0, alpha * 0.8),
        );
    }

    // Head string, foot spot.
    let head = view.to_screen(table.head_spot());
    draw_line(
        head.x,
        felt.y,
        head.x,
        felt.y + felt.h,
        1.0,
        Color::new(1.0, 1.0, 1.0, 0.05),
    );
    let foot = view.to_screen(table.foot_spot());
    draw_circle(foot.x, foot.y, 1.8, Color::new(1.0, 1.0, 1.0, 0.16));
    draw_circle_lines(foot.x, foot.y, 5.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.09));

    // Rail sights, inlaid as small diamonds in the wood.
    let sight_r = (rail * 0.115).max(3.0);
    let sight_inset = RAIL_WIDTH * 0.78;
    for i in 1..8 {
        let x = table.half_length * (i as f64 / 8.0);
        for y in [
            table.half_width + sight_inset,
            -table.half_width - sight_inset,
        ] {
            let spot = view.to_screen(Wv::new(x, y));
            draw_poly(spot.x, spot.y, 4, sight_r, 0.0, fade(p.sight, 0.9));
            draw_poly(
                spot.x,
                spot.y,
                4,
                sight_r * 0.45,
                0.0,
                blend(p.sight, p.wood, 0.45),
            );
        }
    }
    for i in 1..4 {
        let y = table.half_width * (i as f64 / 4.0);
        for x in [
            table.half_length + sight_inset,
            -table.half_length - sight_inset,
        ] {
            let spot = view.to_screen(Wv::new(x, y));
            draw_poly(spot.x, spot.y, 4, sight_r, 0.0, fade(p.sight, 0.9));
            draw_poly(
                spot.x,
                spot.y,
                4,
                sight_r * 0.45,
                0.0,
                blend(p.sight, p.wood, 0.45),
            );
        }
    }

    // Pockets cut through the cloth and the cushion, with a metal liner.
    for pocket in &table.pockets {
        let hole = view.to_screen(pocket.center);
        let r = view.len(pocket.radius);
        draw_circle(hole.x, hole.y, r * 1.42, Color::new(0.0, 0.0, 0.0, 0.08));
        draw_circle(hole.x, hole.y, r * 1.24, Color::new(0.0, 0.0, 0.0, 0.14));
        draw_circle(hole.x, hole.y, r * 1.16, blend(p.wood, white, 0.30));
        draw_circle(hole.x, hole.y, r * 1.08, Color::new(0.09, 0.10, 0.12, 1.0));
        draw_circle(hole.x, hole.y, r * 1.04, Color::new(0.03, 0.033, 0.04, 1.0));
        draw_arc(
            hole.x,
            hole.y,
            16,
            r * 1.10,
            200.0,
            1.8,
            140.0,
            Color::new(1.0, 1.0, 1.0, 0.13),
        );
        draw_circle(hole.x, hole.y, r, Color::new(0.012, 0.013, 0.016, 1.0));
        draw_circle(hole.x, hole.y, r * 0.60, Color::new(0.0, 0.0, 0.0, 1.0));
    }
}

pub fn draw_ball(ball: &Ball, view: &View, highlight: bool) {
    draw_ball_at(ball.id, ball.pos, ball.radius, view, highlight, 1.0);
}

/// Draws one ball at an arbitrary position, radius and opacity.
pub fn draw_ball_at(id: u8, pos: Wv, radius: f64, view: &View, highlight: bool, alpha: f32) {
    let c = view.to_screen(pos);
    let r = view.len(radius);

    if highlight {
        draw_circle(c.x, c.y, r + 4.0, Color::new(1.0, 0.95, 0.4, 0.30 * alpha));
    }

    paint_ball(id, c, r, alpha);
}

/// A ball drawn at a raw screen position, for menus and the HUD tray.
pub fn draw_ball_icon(id: u8, center: Vec2, radius: f32) {
    paint_ball(id, center, radius, 1.0);
}

fn paint_ball(id: u8, c: Vec2, r: f32, alpha: f32) {
    let base = fade(ball_color(id), alpha);
    let white = Color::new(0.97, 0.96, 0.93, 1.0);

    // Soft shadow on the cloth, biased away from the light.
    draw_ellipse(
        c.x + r * 0.12,
        c.y + r * 0.20,
        r * 1.04,
        r * 0.94,
        0.0,
        Color::new(0.0, 0.0, 0.0, 0.26 * alpha),
    );

    if is_striped(id) {
        // White ball with a coloured band around the equator.
        draw_circle(c.x, c.y, r, fade(white, alpha));
        let band = r * 0.58;
        let mut y = -band;
        while y <= band {
            let span = (r * r - y * y).max(0.0).sqrt();
            draw_rectangle(c.x - span, c.y + y, span * 2.0, 1.1, base);
            y += 1.0;
        }
    } else {
        draw_circle(c.x, c.y, r, base);
    }

    // Spherical shading: a vertical terminator, darker towards the bottom.
    let mut y = -r;
    while y <= r {
        let span = (r * r - y * y).max(0.0).sqrt();
        let t = ((y / r) + 1.0) * 0.5;
        let dark = 0.30 * t * t * alpha;
        draw_rectangle(
            c.x - span,
            c.y + y,
            span * 2.0,
            1.0,
            Color::new(0.0, 0.0, 0.0, dark),
        );
        y += 1.0;
    }
    // Slight darkening towards the right, where the light falls off.
    draw_circle_lines(
        c.x + r * 0.05,
        c.y + r * 0.06,
        r * 0.96,
        (r * 0.10).clamp(1.0, 3.0),
        Color::new(0.0, 0.0, 0.0, 0.09 * alpha),
    );

    if id != 0 {
        draw_circle(
            c.x,
            c.y,
            r * 0.42,
            fade(Color::new(0.98, 0.97, 0.94, 1.0), alpha),
        );
        draw_circle_lines(
            c.x,
            c.y,
            r * 0.42,
            1.0,
            Color::new(0.0, 0.0, 0.0, 0.20 * alpha),
        );
        let label = id.to_string();
        let size = (r * 0.90).clamp(7.0, 20.0);
        let metrics = measure_text(&label, None, size as u16, 1.0);
        draw_text(
            &label,
            c.x - metrics.width * 0.5,
            c.y + metrics.height * 0.34,
            size,
            Color::new(0.06, 0.06, 0.07, alpha),
        );
    }

    // Rim, then a glossy highlight in the upper left.
    draw_circle_lines(
        c.x,
        c.y,
        r - 0.4,
        1.0,
        Color::new(0.0, 0.0, 0.0, 0.30 * alpha),
    );
    draw_circle(
        c.x - r * 0.38,
        c.y - r * 0.40,
        r * 0.30,
        Color::new(1.0, 1.0, 1.0, 0.09 * alpha),
    );
    draw_circle(
        c.x - r * 0.40,
        c.y - r * 0.44,
        r * 0.17,
        Color::new(1.0, 1.0, 1.0, 0.14 * alpha),
    );
    draw_circle(
        c.x - r * 0.42,
        c.y - r * 0.46,
        r * 0.08,
        Color::new(1.0, 1.0, 1.0, 0.26 * alpha),
    );
}

pub fn draw_ghost_ball(pos: Wv, radius: f64, view: &View) {
    draw_ghost_ball_tinted(pos, radius, view, Color::new(1.0, 1.0, 1.0, 0.65));
}

pub fn draw_ghost_ball_tinted(pos: Wv, radius: f64, view: &View, outline: Color) {
    let c = view.to_screen(pos);
    let r = view.len(radius);
    draw_circle(c.x, c.y, r, fade(outline, 0.10));
    draw_circle_lines(c.x, c.y, r, 1.5, outline);
    draw_circle(c.x, c.y, 1.6, fade(outline, 0.45));
}

/// A dashed aim line from `from` towards `from + direction * length`.
pub fn draw_aim_line(from: Wv, direction: Wv, length: f64, view: &View) {
    let start = view.to_screen(from);
    let end = view.to_screen(from + direction * length);
    let delta = end - start;
    let total = delta.length();
    if total < 1.0 {
        return;
    }
    let step = 13.0;
    let mut travelled = 0.0;
    while travelled < total {
        let a = start + delta * (travelled / total);
        let b = start + delta * ((travelled + step * 0.55).min(total) / total);
        draw_line(a.x, a.y, b.x, b.y, 5.0, Color::new(1.0, 1.0, 1.0, 0.07));
        draw_line(a.x, a.y, b.x, b.y, 2.0, Color::new(1.0, 1.0, 1.0, 0.72));
        travelled += step;
    }
}

/// A solid aim line with an arrow head, used for the predicted shot.
pub fn draw_shot_line(from: Wv, direction: Wv, length: f64, view: &View, color: Color) {
    if length <= 0.002 {
        return;
    }
    let start = view.to_screen(from);
    let end = view.to_screen(from + direction * length);
    draw_line(start.x, start.y, end.x, end.y, 2.0, color);

    let along = (end - start).normalize_or_zero();
    let side = vec2(-along.y, along.x);
    let wing = 7.0;
    draw_triangle(
        end,
        end - along * wing + side * wing * 0.45,
        end - along * wing - side * wing * 0.45,
        color,
    );
}

/// Draws the cue stick behind the cue ball, pulled back by `pull` metres.
pub fn draw_cue(cue_pos: Wv, direction: Wv, pull: f64, max_back: f64, view: &View) {
    let back = -direction;
    let gap = 0.05 + pull;
    let tip = cue_pos + back * gap;
    let reach = (gap + 0.95).min(max_back.max(gap + 0.18));
    let butt = cue_pos + back * reach;

    let tip_screen = view.to_screen(tip);
    let butt_screen = view.to_screen(butt);
    let length = tip_screen.distance(butt_screen);
    if length < 2.0 {
        return;
    }
    let dir = (butt_screen - tip_screen).normalize();

    // Shadow cast on the cloth.
    draw_line(
        tip_screen.x + 3.0,
        tip_screen.y + 4.0,
        butt_screen.x + 3.0,
        butt_screen.y + 4.0,
        7.0,
        Color::new(0.0, 0.0, 0.0, 0.28),
    );

    // Shaft: pale maple, tapering towards the tip.
    let shaft_end = tip_screen + dir * (length * 0.72);
    draw_line(
        tip_screen.x,
        tip_screen.y,
        shaft_end.x,
        shaft_end.y,
        6.0,
        Color::new(0.86, 0.72, 0.48, 1.0),
    );
    draw_line(
        tip_screen.x + 1.0,
        tip_screen.y,
        shaft_end.x,
        shaft_end.y,
        3.0,
        Color::new(0.95, 0.86, 0.66, 0.55),
    );

    // Butt: dark stained wood with a wrap.
    draw_line(
        shaft_end.x,
        shaft_end.y,
        butt_screen.x,
        butt_screen.y,
        8.0,
        Color::new(0.22, 0.13, 0.09, 1.0),
    );
    let wrap_end = butt_screen - dir * (length * 0.16);
    draw_line(
        shaft_end.x,
        shaft_end.y,
        wrap_end.x,
        wrap_end.y,
        8.5,
        Color::new(0.12, 0.10, 0.10, 1.0),
    );
    draw_circle(
        butt_screen.x,
        butt_screen.y,
        4.2,
        Color::new(0.09, 0.09, 0.09, 1.0),
    );

    // Ferrule and tip.
    draw_line(
        tip_screen.x,
        tip_screen.y,
        tip_screen.x + dir.x * 9.0,
        tip_screen.y + dir.y * 9.0,
        6.0,
        Color::new(0.95, 0.94, 0.90, 1.0),
    );
    draw_line(
        tip_screen.x,
        tip_screen.y,
        tip_screen.x + dir.x * 3.5,
        tip_screen.y + dir.y * 3.5,
        6.0,
        Color::new(0.25, 0.45, 0.80, 1.0),
    );
}

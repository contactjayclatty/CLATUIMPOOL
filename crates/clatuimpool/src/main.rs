#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]

mod render;
mod settings;
mod ui;

use macroquad::prelude::*;
use macroquad::text::{load_ttf_font_from_bytes, set_default_font};
use pool_core::{BALL_RADIUS, Event, Game, Group, ShotOutcome, Table, Vec2 as Wv, group_of};
use render::{
    TableTheme, View, draw_aim_line, draw_ball, draw_ball_at, draw_ball_icon, draw_cue,
    draw_ghost_ball, draw_ghost_ball_tinted, draw_shot_line, draw_table,
};
use settings::Settings;

/// Colour of the predicted path of the object ball.
const OBJECT_LINE: Color = Color::new(1.0, 0.88, 0.35, 0.80);
/// Colour of the predicted deflection of the cue ball.
const CUE_DEFLECT: Color = Color::new(0.75, 0.85, 1.0, 0.45);
/// Ignore pulls shorter than this so a stray click never shoots.
const MIN_PULL: f64 = 0.03;
/// Time a ball takes to slide into a pocket and sink, in seconds.
const DROP_SECONDS: f64 = 0.55;
/// Colour used for the solids group.
const SOLID: Color = Color::new(0.98, 0.85, 0.35, 1.0);
/// Colour used for the stripes group.
const STRIPE: Color = Color::new(0.48, 0.72, 1.0, 1.0);

/// A ball falling into a pocket, played out purely for the eye.
struct Drop {
    id: u8,
    from: Wv,
    to: Wv,
    elapsed: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Menu,
    Playing,
    Paused,
    Settings,
    GameOver,
}

struct App {
    screen: Screen,
    settings: Settings,
    settings_from: Screen,
    game: Game,
    drops: Vec<Drop>,
    dragging: bool,
    dragging_spin: bool,
    power: f64,
    aim: Wv,
    /// Cue ball spin: `x` is side, `y` is follow, both in `-1..=1`.
    spin: Wv,
    log: Vec<String>,
    shake: f32,
    time: f32,
    fps: f32,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "CLATUIMPOOL - 8-Ball".to_owned(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    install_ui_font();
    // Headless visual check: `CLATUIMPOOL_SHOT=<dir>` renders a few screens,
    // writes them as PNG and exits. Used to review the look without a desktop.
    let shots = std::env::var("CLATUIMPOOL_SHOT").ok();
    if shots.is_some() {
        app.start_match();
    }
    let mut frame = 0u32;
    loop {
        let dt = get_frame_time() as f64;
        app.frame(dt);

        if let Some(dir) = &shots {
            frame += 1;
            match frame {
                40 => get_screen_data().export_png(&format!("{dir}\\play.png")),
                44 => app.screen = Screen::Menu,
                48 => get_screen_data().export_png(&format!("{dir}\\menu.png")),
                52 => {
                    app.settings_from = Screen::Menu;
                    app.screen = Screen::Settings;
                }
                56 => get_screen_data().export_png(&format!("{dir}\\settings.png")),
                60 => std::process::exit(0),
                _ => {}
            }
        }

        next_frame().await;
    }
}

/// The built-in macroquad face is a thin bitmap font that makes the whole
/// interface look unfinished, so prefer a system UI font when one is present.
fn install_ui_font() {
    for path in [
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\SegoeUI.ttf",
        r"C:\Windows\Fonts\arial.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(font) = load_ttf_font_from_bytes(&bytes)
        {
            set_default_font(font);
            return;
        }
    }
}

impl App {
    fn new() -> Self {
        let settings = Settings::load();
        if settings.fullscreen {
            set_fullscreen(true);
        }
        Self {
            screen: Screen::Menu,
            settings,
            settings_from: Screen::Menu,
            game: Game::new(),
            drops: Vec::new(),
            dragging: false,
            dragging_spin: false,
            power: 0.0,
            aim: Wv::ZERO,
            spin: Wv::ZERO,
            log: Vec::new(),
            shake: 0.0,
            time: 0.0,
            fps: 60.0,
        }
    }

    fn frame(&mut self, dt: f64) {
        self.time += dt as f32;
        self.fps = self.fps * 0.92 + (1.0 / dt.max(1e-4) as f32) * 0.08;
        if !is_mouse_button_down(MouseButton::Left) {
            self.dragging_spin = false;
        }

        match self.screen {
            Screen::Menu => self.menu_frame(),
            Screen::Settings => self.settings_frame(),
            Screen::Playing => self.play_frame(dt),
            Screen::Paused => {
                self.draw_world();
                self.pause_frame();
            }
            Screen::GameOver => {
                self.draw_world();
                self.over_frame();
            }
        }

        if self.settings.show_fps {
            draw_text(
                format!("{:.0} fps", self.fps),
                screen_width() - 84.0,
                20.0,
                15.0,
                ui::DIM,
            );
        }
    }

    // ---------------------------------------------------------------- screens

    fn menu_frame(&mut self) {
        clear_background(ui::BG);
        let w = screen_width();
        let h = screen_height();
        for band in 0..14 {
            let t = band as f32 / 14.0;
            draw_rectangle(
                0.0,
                h * t,
                w,
                h / 14.0 + 1.0,
                Color::new(0.03, 0.05, 0.06, t * 0.35),
            );
        }

        let cx = w * 0.5;
        for step in 0..30 {
            let t = step as f32 / 30.0;
            let radius = 150.0 - t * 96.0;
            draw_circle(cx, 148.0, radius, Color::new(0.10, 0.42, 0.26, 0.012));
        }
        draw_ball_icon(8, vec2(cx, 148.0), 52.0);

        ui::centered("CLATUIMPOOL", cx, 292.0, 58.0, ui::TEXT);
        ui::rounded(cx - 46.0, 306.0, 92.0, 3.0, 1.5, ui::ACCENT);
        ui::centered("eight ball pool", cx, 336.0, 20.0, ui::DIM);

        let bw = 280.0;
        let bx = cx - bw * 0.5;
        if ui::button(
            Rect::new(bx, 372.0, bw, 56.0),
            "PLAY",
            ui::Kind::Primary,
            23.0,
        ) {
            self.start_match();
        }
        if ui::button(
            Rect::new(bx, 444.0, bw, 52.0),
            "SETTINGS",
            ui::Kind::Normal,
            20.0,
        ) {
            self.settings_from = Screen::Menu;
            self.screen = Screen::Settings;
        }
        if ui::button(
            Rect::new(bx, 512.0, bw, 52.0),
            "QUIT",
            ui::Kind::Normal,
            20.0,
        ) {
            self.settings.save();
            std::process::exit(0);
        }

        ui::centered(
            "hot seat  -  drag the cue back and release",
            cx,
            h - 34.0,
            16.0,
            ui::DIM,
        );
    }

    fn settings_frame(&mut self) {
        clear_background(ui::BG);
        let w = 780.0;
        let h = 648.0;
        let x = (screen_width() - w) * 0.5;
        let y = (screen_height() - h) * 0.5;
        ui::rounded(x, y, w, h, 16.0, ui::PANEL);
        ui::outline(x, y, w, h, ui::EDGE);
        ui::centered("SETTINGS", x + w * 0.5, y + 44.0, 28.0, ui::TEXT);
        draw_line(x + 24.0, y + 60.0, x + w - 24.0, y + 60.0, 1.0, ui::EDGE);

        let row_h = 40.0;
        let rx = x + 24.0;
        let rw = w - 48.0;
        let row = |index: usize| Rect::new(rx, y + 66.0 + index as f32 * row_h, rw, row_h);

        let mut s = self.settings.clone();
        s.aim_guide = ui::toggle(row(0), "Aim guide", s.aim_guide);
        s.object_lines = ui::toggle(row(1), "Predicted ball lines", s.object_lines);
        s.ghost_ball = ui::toggle(row(2), "Ghost ball", s.ghost_ball);
        s.show_spin = ui::toggle(row(3), "Spin pad", s.show_spin);
        s.show_tray = ui::toggle(row(4), "Potted balls tray", s.show_tray);
        s.show_log = ui::toggle(row(5), "Shot commentary", s.show_log);
        s.screen_shake = ui::toggle(row(6), "Screen shake", s.screen_shake);
        s.slow_motion = ui::toggle(row(7), "Slow motion", s.slow_motion);
        s.fullscreen = ui::toggle(row(8), "Fullscreen", s.fullscreen);
        s.sink_speed = ui::slider(
            row(9),
            "Sink speed",
            s.sink_speed,
            0.4,
            2.5,
            &format!("{:.2}x", s.sink_speed),
        );
        s.drag_distance = ui::slider(
            row(10),
            "Pull for full power",
            s.drag_distance,
            0.2,
            0.8,
            &format!("{:.2} m", s.drag_distance),
        );
        let labels = TableTheme::ALL.map(TableTheme::label);
        s.theme = TableTheme::from_index(ui::segmented(row(11), "Table", &labels, s.theme.index()));

        if s != self.settings {
            if s.fullscreen != self.settings.fullscreen {
                set_fullscreen(s.fullscreen);
            }
            self.settings = s;
            self.settings.save();
        }

        if ui::button(
            Rect::new(x + 24.0, y + h - 56.0, 160.0, 38.0),
            "RESET",
            ui::Kind::Danger,
            17.0,
        ) {
            self.settings = Settings::default();
            self.settings.save();
        }
        if ui::button(
            Rect::new(x + w - 164.0, y + h - 56.0, 140.0, 38.0),
            "BACK",
            ui::Kind::Normal,
            17.0,
        ) || is_key_pressed(KeyCode::Escape)
        {
            self.screen = self.settings_from;
        }
    }

    fn play_frame(&mut self, frame_dt: f64) {
        let view = View::new(screen_width(), screen_height());
        let mouse_px = vec2(mouse_position().0, mouse_position().1);
        let mouse = view.to_world(mouse_px);

        if is_key_pressed(KeyCode::Escape) {
            self.dragging = false;
            self.power = 0.0;
            self.screen = Screen::Paused;
            return;
        }
        if is_key_pressed(KeyCode::R) {
            self.start_match();
        }

        self.handle_aim(&view, mouse, mouse_px);

        let speed = if self.settings.slow_motion { 0.45 } else { 1.0 };
        self.game.update(frame_dt * speed);
        self.collect_events();

        for drop in &mut self.drops {
            drop.elapsed += frame_dt;
        }
        let life = DROP_SECONDS / self.settings.sink_speed.max(0.25) as f64;
        self.drops.retain(|drop| drop.elapsed < life);
        self.shake = (self.shake - frame_dt as f32 * 2.2).max(0.0);

        self.draw_world();
    }

    fn pause_frame(&mut self) {
        let w = screen_width();
        let h = screen_height();
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.02, 0.03, 0.04, 0.70));
        let pw = 340.0;
        let ph = 356.0;
        let x = (w - pw) * 0.5;
        let y = (h - ph) * 0.5;
        ui::rounded(x, y, pw, ph, 16.0, ui::PANEL);
        ui::outline(x, y, pw, ph, ui::EDGE);
        ui::centered("PAUSED", x + pw * 0.5, y + 52.0, 30.0, ui::TEXT);

        let bx = x + 30.0;
        let bw = pw - 60.0;
        if ui::button(
            Rect::new(bx, y + 78.0, bw, 48.0),
            "RESUME",
            ui::Kind::Primary,
            20.0,
        ) || is_key_pressed(KeyCode::Escape)
        {
            self.screen = Screen::Playing;
        }
        if ui::button(
            Rect::new(bx, y + 138.0, bw, 46.0),
            "SETTINGS",
            ui::Kind::Normal,
            19.0,
        ) {
            self.settings_from = Screen::Paused;
            self.screen = Screen::Settings;
        }
        if ui::button(
            Rect::new(bx, y + 194.0, bw, 46.0),
            "NEW RACK",
            ui::Kind::Normal,
            19.0,
        ) {
            self.start_match();
        }
        if ui::button(
            Rect::new(bx, y + 250.0, bw, 46.0),
            "MAIN MENU",
            ui::Kind::Danger,
            19.0,
        ) {
            self.screen = Screen::Menu;
        }
    }

    fn over_frame(&mut self) {
        let w = screen_width();
        let h = screen_height();
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.02, 0.03, 0.04, 0.78));
        let pw = 440.0;
        let ph = 320.0;
        let x = (w - pw) * 0.5;
        let y = (h - ph) * 0.5;
        ui::rounded(x, y, pw, ph, 16.0, ui::PANEL);
        ui::outline(x, y, pw, ph, ui::EDGE);

        let winner = self.game.rules.winner.unwrap_or(0);
        ui::centered("GAME OVER", x + pw * 0.5, y + 56.0, 26.0, ui::DIM);
        ui::centered(
            &format!("PLAYER {} WINS", winner + 1),
            x + pw * 0.5,
            y + 104.0,
            40.0,
            ui::GOLD,
        );
        ui::centered(
            &format!(
                "{} shots  -  {} balls potted",
                self.game.shots_taken,
                self.game.rules.potted_by(winner).len()
            ),
            x + pw * 0.5,
            y + 142.0,
            18.0,
            ui::DIM,
        );

        let bw = pw - 80.0;
        let bx = x + 40.0;
        if ui::button(
            Rect::new(bx, y + 178.0, bw, 52.0),
            "PLAY AGAIN",
            ui::Kind::Primary,
            21.0,
        ) || is_key_pressed(KeyCode::Enter)
        {
            self.start_match();
        }
        if ui::button(
            Rect::new(bx, y + 240.0, bw, 46.0),
            "MAIN MENU",
            ui::Kind::Normal,
            19.0,
        ) {
            self.screen = Screen::Menu;
        }
    }

    fn start_match(&mut self) {
        self.game.rack();
        self.drops.clear();
        self.log.clear();
        self.dragging = false;
        self.power = 0.0;
        self.aim = Wv::ZERO;
        self.spin = Wv::ZERO;
        self.shake = 0.0;
        self.screen = Screen::Playing;
    }

    // ------------------------------------------------------------------ input

    fn handle_aim(&mut self, view: &View, mouse: Wv, mouse_px: Vec2) {
        if self.settings.show_spin && self.spin_pad_rect().contains(mouse_px) {
            self.dragging = false;
            self.power = 0.0;
            if is_mouse_button_pressed(MouseButton::Left) {
                self.dragging_spin = true;
            }
            if self.dragging_spin && is_mouse_button_down(MouseButton::Left) {
                let c = self.spin_pad_rect().center();
                let r = self.spin_pad_rect().w * 0.42;
                let offset = (mouse_px - c) / r;
                let mut side = offset.x;
                let mut follow = -offset.y;
                let length = (side * side + follow * follow).sqrt();
                if length > 1.0 {
                    side /= length;
                    follow /= length;
                }
                self.spin = Wv::new(side as f64, follow as f64);
            }
            return;
        }

        if !self.game.ready() {
            self.dragging = false;
            self.power = 0.0;
            return;
        }

        if self.game.rules.ball_in_hand {
            self.dragging = false;
            self.power = 0.0;
            if is_mouse_button_pressed(MouseButton::Left) && self.on_table(view, mouse_px) {
                self.game.place_cue_ball(mouse);
            }
            return;
        }

        let cue = self.game.cue_ball().pos;
        if is_mouse_button_pressed(MouseButton::Left) && self.on_table(view, mouse_px) {
            self.dragging = true;
        }

        let pull_limit = self.settings.drag_distance as f64;
        let pull = mouse.distance(cue).min(pull_limit);
        self.aim = (cue - mouse).normalized();

        if !self.dragging {
            self.power = 0.0;
            return;
        }
        // Softer response on short pulls, full power only at a full pull.
        self.power = (pull / pull_limit).powf(1.4);
        if is_mouse_button_released(MouseButton::Left) {
            if pull > MIN_PULL && !self.aim.is_zero() {
                self.game
                    .shoot_spin(self.aim, self.power, self.spin.y, self.spin.x);
            }
            self.dragging = false;
            self.power = 0.0;
        }
    }

    fn on_table(&self, view: &View, mouse_px: Vec2) -> bool {
        let top = 92.0;
        let bottom = screen_height() - 100.0;
        mouse_px.y > top && mouse_px.y < bottom && view.to_world(mouse_px).x.abs() < 1.4
    }

    fn collect_events(&mut self) {
        let events = self.game.take_events();
        for event in events {
            match event {
                Event::BallHit { speed, .. } => {
                    if self.settings.screen_shake && speed > 1.1 {
                        self.shake = (self.shake + (speed as f32 * 0.06).min(0.5)).min(1.0);
                    }
                }
                Event::Potted { ball, pocket } => {
                    if let Some(dropped) = self.game.balls.iter().find(|b| b.id == ball) {
                        let from = dropped.potted_at.unwrap_or(dropped.pos);
                        self.drops.push(Drop {
                            id: ball,
                            from,
                            to: self.game.table.pockets[pocket].center,
                            elapsed: 0.0,
                        });
                    }
                    if self.settings.screen_shake {
                        self.shake = (self.shake + 0.15).min(1.0);
                    }
                    if ball == 0 {
                        self.push("cue ball potted");
                    } else if ball == 8 {
                        self.push("the eight drops");
                    } else {
                        self.push(format!("ball {ball} potted"));
                    }
                }
                Event::ShotResolved { outcome } => match outcome {
                    ShotOutcome::Continue => self.push("pot - shoot again"),
                    ShotOutcome::TurnOver => self.push("no pot - turn over"),
                    ShotOutcome::Foul(reason) => {
                        self.push(format!("foul: {}", reason.label()));
                    }
                    ShotOutcome::Win { player } => {
                        self.push(format!("player {} sinks the eight", player + 1));
                    }
                    ShotOutcome::Lose { player } => {
                        self.push(format!("player {} sinks the eight early", player + 1));
                    }
                },
                Event::GameOver { winner } => {
                    self.push(format!("player {} wins", winner + 1));
                    self.screen = Screen::GameOver;
                    self.dragging = false;
                    self.power = 0.0;
                }
                _ => {}
            }
        }
    }

    fn push(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
        if self.log.len() > 5 {
            self.log.remove(0);
        }
    }

    // ------------------------------------------------------------------ scene

    fn view(&self) -> View {
        let mut view = View::new(screen_width(), screen_height());
        if self.settings.screen_shake && self.shake > 0.01 {
            let amp = self.shake * 5.0;
            view.center += vec2(
                (self.time * 89.0).sin() * amp,
                (self.time * 71.0).cos() * amp,
            );
        }
        view
    }

    fn draw_world(&self) {
        let view = self.view();
        clear_background(ui::BG);
        draw_table(&self.game.table, &view, self.settings.theme);

        if self.aim_visible() {
            self.draw_aim(&view);
        }

        for ball in self.game.balls.iter().filter(|ball| ball.is_active()) {
            draw_ball(ball, &view, ball.id == 0 && self.game.rules.ball_in_hand);
        }
        self.draw_drops(&view);

        if self.game.rules.ball_in_hand && self.game.ready() && self.screen == Screen::Playing {
            let px = vec2(mouse_position().0, mouse_position().1);
            if self.on_table(&view, px) {
                let spot = view.to_world(px);
                let color = if self.game.spot_is_free(spot) {
                    Color::new(0.55, 0.95, 0.62, 0.85)
                } else {
                    Color::new(0.95, 0.38, 0.36, 0.85)
                };
                draw_ghost_ball_tinted(spot, BALL_RADIUS, &view, color);
            }
        }

        self.draw_hud();
    }

    fn aim_visible(&self) -> bool {
        self.screen == Screen::Playing
            && self.game.ready()
            && !self.game.rules.ball_in_hand
            && !self.aim.is_zero()
    }

    fn draw_aim(&self, view: &View) {
        let prediction = predict(&self.game, self.aim);
        let cue_pos = self.game.cue_ball().pos;

        if self.settings.aim_guide && prediction.distance > 0.0 {
            draw_aim_line(cue_pos, self.aim, prediction.distance, view);
        }

        if let Some(contact) = prediction.contact {
            if self.settings.ghost_ball {
                draw_ghost_ball(contact, self.game.cue_ball().radius, view);
            }
            if self.settings.object_lines
                && let Some(target) = prediction.target
                && let Some(ball) = self.game.balls.iter().find(|ball| ball.id == target)
            {
                let object_dir = (ball.pos - contact).normalized();
                if !object_dir.is_zero() {
                    let length = ray_hit(&self.game, ball.pos, object_dir, ball.radius, ball.id);
                    draw_shot_line(ball.pos, object_dir, length, view, OBJECT_LINE);
                }

                let tangent = self.aim - object_dir * self.aim.dot(object_dir);
                let tangent_dir = tangent.normalized();
                if !tangent_dir.is_zero() {
                    let length =
                        ray_hit(&self.game, contact, tangent_dir, ball.radius, 0).min(0.75);
                    draw_shot_line(contact, tangent_dir, length, view, CUE_DEFLECT);
                }
            }
        }

        draw_cue(
            cue_pos,
            self.aim,
            self.power * self.settings.drag_distance as f64,
            distance_to_rail(&self.game.table, cue_pos, -self.aim, 0.0) - 0.03,
            view,
        );
    }

    /// Slides the ball the last stretch into the pocket, then sinks it below
    /// the rim with a ripple on the cloth.
    fn draw_drops(&self, view: &View) {
        for drop in &self.drops {
            let t = (drop.elapsed * self.settings.sink_speed.max(0.25) as f64 / DROP_SECONDS)
                .clamp(0.0, 1.0);
            let slide = (t / 0.42).min(1.0);
            let ease = slide * slide;
            let pos = drop.from + (drop.to - drop.from) * ease;
            let sink = ((t - 0.42) / 0.58).clamp(0.0, 1.0);
            let radius = BALL_RADIUS * (1.0 - 0.82 * sink);
            let alpha = (1.0 - sink * sink) as f32;
            draw_ball_at(drop.id, pos, radius, view, false, alpha.clamp(0.0, 1.0));

            let pocket = view.to_screen(drop.to);
            let mouth = view.len(0.05);
            let ring = ((t * 1.35).min(1.0)) as f32;
            draw_circle_lines(
                pocket.x,
                pocket.y,
                view.len(BALL_RADIUS) * 0.6 + mouth * ring,
                2.0,
                Color::new(1.0, 0.95, 0.75, (1.0 - ring) * 0.35),
            );

            if t > 0.50 {
                let dark = ((t - 0.50) / 0.18).min(1.0) as f32;
                draw_circle(
                    pocket.x,
                    pocket.y,
                    view.len(BALL_RADIUS) * 1.02,
                    Color::new(0.0, 0.0, 0.0, dark),
                );
            }
        }
    }

    // -------------------------------------------------------------------- hud

    fn draw_hud(&self) {
        let w = screen_width();
        let h = screen_height();
        let bar = 80.0;

        draw_rectangle(0.0, 0.0, w, bar, Color::new(0.035, 0.045, 0.058, 0.94));
        draw_line(0.0, bar, w, bar, 1.0, ui::EDGE);
        self.player_card(20.0, 8.0, 336.0, 64.0, 0);
        self.player_card(w - 356.0, 8.0, 336.0, 64.0, 1);

        let turn = if self.game.rules.winner.is_some() {
            "game over".to_owned()
        } else if self.game.rules.ball_in_hand {
            "ball in hand".to_owned()
        } else {
            format!("PLAYER {} TO SHOOT", self.game.rules.current_player + 1)
        };
        ui::centered(
            &format!("SHOT {}", self.game.shots_taken + 1),
            w * 0.5,
            30.0,
            15.0,
            ui::DIM,
        );
        ui::centered(&turn, w * 0.5, 54.0, 21.0, ui::TEXT);

        let bottom = 92.0;
        let by = h - bottom;
        draw_rectangle(0.0, by, w, bottom, Color::new(0.035, 0.045, 0.058, 0.94));
        draw_line(0.0, by, w, by, 1.0, ui::EDGE);

        if self.settings.show_tray {
            self.draw_tray(by);
        }

        let hint = if self.game.rules.ball_in_hand {
            "click the table to place the cue ball"
        } else if !self.game.ready() {
            "balls rolling"
        } else {
            "drag back from the cue and release"
        };
        ui::centered(hint, w * 0.5, by + 28.0, 15.0, ui::DIM);
        ui::centered(
            "R new rack   -   Esc menu",
            w * 0.5,
            by + 50.0,
            14.0,
            Color::new(0.42, 0.45, 0.52, 1.0),
        );

        if self.dragging && self.power > 0.0 {
            self.draw_power_bar();
        }
        if self.settings.show_spin {
            self.draw_spin_pad();
        }
        if self.settings.show_log {
            self.draw_log(by);
        }

        if self.game.rules.ball_in_hand && self.game.rules.winner.is_none() {
            let text = "BALL IN HAND";
            let size = 20.0;
            let metrics = measure_text(text, None, size as u16, 1.0);
            ui::rounded(
                w * 0.5 - metrics.width * 0.5 - 18.0,
                bar + 14.0,
                metrics.width + 36.0,
                34.0,
                8.0,
                ui::GOLD,
            );
            draw_text(
                text,
                w * 0.5 - metrics.width * 0.5,
                bar + 38.0,
                size,
                Color::new(0.10, 0.08, 0.02, 1.0),
            );
        }
    }

    fn player_card(&self, x: f32, y: f32, w: f32, h: f32, player: usize) {
        let active = self.game.rules.current_player == player
            && self.game.rules.winner.is_none()
            && self.screen == Screen::Playing;
        let fill = if active {
            Color::new(0.09, 0.16, 0.12, 0.97)
        } else {
            ui::PANEL
        };
        ui::rounded(x, y, w, h, 10.0, fill);
        ui::outline(x, y, w, h, if active { ui::ACCENT } else { ui::EDGE });
        if active {
            ui::rounded(x + 2.0, y + 10.0, 4.0, h - 20.0, 2.0, ui::ACCENT);
        }

        let name_x = if active {
            draw_circle(x + 18.0, y + 22.0, 4.5, ui::ACCENT);
            x + 30.0
        } else {
            x + 16.0
        };
        draw_text(
            format!("PLAYER {}", player + 1),
            name_x,
            y + 26.0,
            20.0,
            if active { ui::TEXT } else { ui::DIM },
        );

        let line = match self.game.rules.group_of_player(player) {
            Some(Group::Solids) => format!(
                "SOLIDS  -  {} LEFT",
                self.game.remaining_in_group(Group::Solids)
            ),
            Some(Group::Stripes) => format!(
                "STRIPES  -  {} LEFT",
                self.game.remaining_in_group(Group::Stripes)
            ),
            None => "OPEN TABLE".to_owned(),
        };
        let color = match self.game.rules.group_of_player(player) {
            Some(Group::Solids) => SOLID,
            Some(Group::Stripes) => STRIPE,
            None => ui::DIM,
        };
        draw_text(&line, x + 16.0, y + 50.0, 15.0, color);

        let remaining = remaining_balls(&self.game, player);
        let count = remaining.len();
        for (index, id) in remaining.iter().enumerate() {
            let cx = x + w - 18.0 - (count - 1 - index) as f32 * 19.0;
            draw_ball_icon(*id, vec2(cx, y + 23.0), 8.0);
        }
    }

    fn draw_tray(&self, by: f32) {
        let player = self.game.rules.current_player;
        let potted = self.game.rules.potted_by(player);
        draw_text("POTTED", 26.0, by + 22.0, 13.0, ui::DIM);
        draw_text(
            format!("PLAYER {}", player + 1),
            88.0,
            by + 22.0,
            13.0,
            ui::TEXT,
        );
        if potted.is_empty() {
            draw_text("nothing yet", 26.0, by + 60.0, 16.0, ui::DIM);
            return;
        }
        for (index, id) in potted.iter().take(9).enumerate() {
            draw_ball_icon(*id, vec2(42.0 + index as f32 * 38.0, by + 56.0), 16.0);
        }
    }

    fn draw_log(&self, by: f32) {
        for (index, line) in self.log.iter().enumerate() {
            let y = by - 16.0 - (self.log.len() - 1 - index) as f32 * 22.0;
            draw_text(line, 26.0, y, 17.0, Color::new(0.82, 0.86, 0.91, 0.92));
        }
    }

    fn draw_power_bar(&self) {
        let x = 26.0;
        let bar_height = 230.0;
        let y = (screen_height() * 0.5 - bar_height * 0.5).max(110.0);
        let ratio = self.power.clamp(0.0, 1.0) as f32;

        ui::rounded(
            x,
            y,
            22.0,
            bar_height,
            8.0,
            Color::new(0.06, 0.07, 0.09, 0.90),
        );
        let color = Color::new(0.55 + 0.40 * ratio, 0.45, 0.15, 0.98);
        ui::rounded(
            x + 3.0,
            y + 3.0 + (bar_height - 6.0) * (1.0 - ratio),
            16.0,
            (bar_height - 6.0) * ratio,
            6.0,
            color,
        );
        ui::outline(x, y, 22.0, bar_height, ui::EDGE);
        ui::centered(
            &format!("{:.0}%", self.power * 100.0),
            x + 11.0,
            y + bar_height + 22.0,
            16.0,
            ui::TEXT,
        );
    }

    fn spin_pad_rect(&self) -> Rect {
        let r = 78.0;
        Rect::new(screen_width() - r - 26.0, screen_height() - 96.0, r, r)
    }

    fn draw_spin_pad(&self) {
        let rect = self.spin_pad_rect();
        let c = rect.center();
        let r = rect.w * 0.42;

        draw_circle(c.x, c.y, r + 5.0, Color::new(0.0, 0.0, 0.0, 0.40));
        draw_circle(c.x, c.y, r, Color::new(0.93, 0.92, 0.88, 1.0));
        draw_circle_lines(c.x, c.y, r, 1.2, Color::new(0.0, 0.0, 0.0, 0.18));
        draw_line(
            c.x - r * 0.78,
            c.y,
            c.x + r * 0.78,
            c.y,
            1.0,
            Color::new(0.0, 0.0, 0.0, 0.12),
        );
        draw_line(
            c.x,
            c.y - r * 0.78,
            c.x,
            c.y + r * 0.78,
            1.0,
            Color::new(0.0, 0.0, 0.0, 0.12),
        );

        let dot = vec2(
            c.x + self.spin.x as f32 * r * 0.74,
            c.y - self.spin.y as f32 * r * 0.74,
        );
        draw_circle(dot.x, dot.y, 6.5, Color::new(0.86, 0.18, 0.22, 1.0));
        draw_circle_lines(dot.x, dot.y, 6.5, 1.0, Color::new(0.0, 0.0, 0.0, 0.35));

        ui::centered("SPIN", c.x, c.y + r + 20.0, 12.0, ui::DIM);
    }
}

/// The balls a player still has to pot, in ascending order. Empty while the
/// table is open, because neither player owns a group yet.
fn remaining_balls(game: &Game, player: usize) -> Vec<u8> {
    let Some(group) = game.rules.group_of_player(player) else {
        return Vec::new();
    };
    let mut balls = game
        .balls
        .iter()
        .filter(|ball| ball.is_active() && group_of(ball.id) == Some(group))
        .map(|ball| ball.id)
        .collect::<Vec<_>>();
    balls.sort_unstable();
    balls
}

/// Where the cue ball would travel and what it would hit first.
struct Prediction {
    distance: f64,
    contact: Option<Wv>,
    target: Option<u8>,
}

fn predict(game: &Game, direction: Wv) -> Prediction {
    let cue = game.cue_ball();
    let mut distance = distance_to_rail(&game.table, cue.pos, direction, cue.radius);
    let mut contact = None;
    let mut target = None;

    for ball in game
        .balls
        .iter()
        .filter(|ball| ball.id != 0 && ball.is_active())
    {
        let relative = ball.pos - cue.pos;
        let along = relative.dot(direction);
        if along <= 0.0 {
            continue;
        }
        let perpendicular_sq = relative.length_squared() - along * along;
        let reach = 2.0 * ball.radius;
        if perpendicular_sq > reach * reach {
            continue;
        }
        let t = along - (reach * reach - perpendicular_sq).max(0.0).sqrt();
        if t >= 0.0 && t < distance {
            distance = t;
            contact = Some(cue.pos + direction * t);
            target = Some(ball.id);
        }
    }

    Prediction {
        distance,
        contact,
        target,
    }
}

/// Distance from `from` to the first ball or cushion along `direction`.
fn ray_hit(game: &Game, from: Wv, direction: Wv, radius: f64, ignore: u8) -> f64 {
    let mut best = distance_to_rail(&game.table, from, direction, radius);
    for ball in game
        .balls
        .iter()
        .filter(|ball| ball.is_active() && ball.id != ignore)
    {
        let relative = ball.pos - from;
        let along = relative.dot(direction);
        if along <= 0.0 {
            continue;
        }
        let perpendicular_sq = relative.length_squared() - along * along;
        let reach = radius + ball.radius;
        if perpendicular_sq > reach * reach {
            continue;
        }
        let t = along - (reach * reach - perpendicular_sq).max(0.0).sqrt();
        if t >= 0.0 && t < best {
            best = t;
        }
    }
    best
}

fn distance_to_rail(table: &Table, pos: Wv, direction: Wv, radius: f64) -> f64 {
    let hx = table.half_length - radius;
    let hy = table.half_width - radius;
    let mut best = f64::INFINITY;
    if direction.x > 1e-9 {
        best = best.min((hx - pos.x) / direction.x);
    } else if direction.x < -1e-9 {
        best = best.min((-hx - pos.x) / direction.x);
    }
    if direction.y > 1e-9 {
        best = best.min((hy - pos.y) / direction.y);
    } else if direction.y < -1e-9 {
        best = best.min((-hy - pos.y) / direction.y);
    }
    best.max(0.0)
}

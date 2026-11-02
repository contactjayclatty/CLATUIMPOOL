use crate::render::TableTheme;

/// Where the settings live, next to the executable's working directory.
const FILE: &str = "clatuimpool.cfg";

/// Everything the player can tune from the settings screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Dashed line from the cue ball to the first contact.
    pub aim_guide: bool,
    /// Predicted lines for the object ball and the cue ball's deflection.
    pub object_lines: bool,
    /// Ring showing where the cue ball will strike.
    pub ghost_ball: bool,
    /// Balls each player has potted, in the HUD.
    pub show_tray: bool,
    /// Rolling shot commentary.
    pub show_log: bool,
    /// Frame rate in the corner.
    pub show_fps: bool,
    /// Cue ball spin pad.
    pub show_spin: bool,
    /// Camera kick when balls collide.
    pub screen_shake: bool,
    pub fullscreen: bool,
    /// Half speed simulation, for lining shots up.
    pub slow_motion: bool,
    /// 0.5 = slow sink, 2.0 = instant.
    pub sink_speed: f32,
    /// Cue pull distance, in metres, that counts as full power.
    pub drag_distance: f32,
    pub theme: TableTheme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            aim_guide: true,
            object_lines: true,
            ghost_ball: true,
            show_tray: true,
            show_log: true,
            show_fps: false,
            show_spin: true,
            screen_shake: true,
            fullscreen: false,
            slow_motion: false,
            sink_speed: 1.0,
            drag_distance: 0.45,
            theme: TableTheme::Classic,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut settings = Self::default();
        let Ok(text) = std::fs::read_to_string(FILE) else {
            return settings;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            let flag = value == "true" || value == "1";
            match key {
                "aim_guide" => settings.aim_guide = flag,
                "object_lines" => settings.object_lines = flag,
                "ghost_ball" => settings.ghost_ball = flag,
                "show_tray" => settings.show_tray = flag,
                "show_log" => settings.show_log = flag,
                "show_fps" => settings.show_fps = flag,
                "show_spin" => settings.show_spin = flag,
                "screen_shake" => settings.screen_shake = flag,
                "fullscreen" => settings.fullscreen = flag,
                "slow_motion" => settings.slow_motion = flag,
                "sink_speed" => {
                    if let Ok(parsed) = value.parse::<f32>() {
                        settings.sink_speed = parsed.clamp(0.25, 3.0);
                    }
                }
                "drag_distance" => {
                    if let Ok(parsed) = value.parse::<f32>() {
                        settings.drag_distance = parsed.clamp(0.15, 0.9);
                    }
                }
                "theme" => {
                    if let Ok(parsed) = value.parse::<usize>() {
                        settings.theme = TableTheme::from_index(parsed);
                    }
                }
                _ => {}
            }
        }
        settings
    }

    pub fn save(&self) {
        let body = format!(
            "aim_guide={}\nobject_lines={}\nghost_ball={}\nshow_tray={}\nshow_log={}\n\
             show_fps={}\nshow_spin={}\nscreen_shake={}\nfullscreen={}\nslow_motion={}\n\
             sink_speed={}\ndrag_distance={}\ntheme={}\n",
            self.aim_guide,
            self.object_lines,
            self.ghost_ball,
            self.show_tray,
            self.show_log,
            self.show_fps,
            self.show_spin,
            self.screen_shake,
            self.fullscreen,
            self.slow_motion,
            self.sink_speed,
            self.drag_distance,
            self.theme.index(),
        );
        let _ = std::fs::write(FILE, body);
    }
}

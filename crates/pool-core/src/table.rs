use crate::ball::Ball;
use crate::vec2::Vec2;

/// Playing surface length in metres (a 9 ft table), along the x axis.
pub const PLAY_LENGTH: f64 = 2.54;
/// Playing surface width in metres, along the y axis.
pub const PLAY_WIDTH: f64 = 1.27;

/// A pocket mouth, modelled as a disc centred on the cushion line.
#[derive(Clone, Copy, Debug)]
pub struct Pocket {
    pub center: Vec2,
    /// Radius of the hole. A ball whose centre is inside it drops, which keeps
    /// the physics and the drawing in agreement: nothing can rest on the rim.
    pub radius: f64,
}

/// The playing surface: a rectangle centred on the origin plus six pockets.
#[derive(Clone, Debug)]
pub struct Table {
    pub half_length: f64,
    pub half_width: f64,
    pub pockets: Vec<Pocket>,
}

impl Default for Table {
    fn default() -> Self {
        Self::standard()
    }
}

impl Table {
    /// A regulation-proportioned 9 ft table with corner and side pockets.
    pub fn standard() -> Self {
        let half_width = PLAY_WIDTH / 2.0;
        let half_length = PLAY_LENGTH / 2.0;
        const CORNER: f64 = 0.070;
        const MIDDLE: f64 = 0.066;

        let pockets = vec![
            Pocket {
                center: Vec2::new(-half_length, -half_width),
                radius: CORNER,
            },
            Pocket {
                center: Vec2::new(-half_length, half_width),
                radius: CORNER,
            },
            Pocket {
                center: Vec2::new(half_length, -half_width),
                radius: CORNER,
            },
            Pocket {
                center: Vec2::new(half_length, half_width),
                radius: CORNER,
            },
            Pocket {
                center: Vec2::new(0.0, -half_width),
                radius: MIDDLE,
            },
            Pocket {
                center: Vec2::new(0.0, half_width),
                radius: MIDDLE,
            },
        ];

        Self {
            half_length,
            half_width,
            pockets,
        }
    }

    /// Returns the index of the pocket that would swallow a ball centred at `pos`.
    pub fn captures(&self, pos: Vec2) -> Option<usize> {
        self.pockets
            .iter()
            .position(|pocket| pos.distance(pocket.center) <= pocket.radius)
    }

    /// True when `pos` sits inside a pocket mouth, where the cushions are cut
    /// away. A ball there is on its way into the pocket, not against a rail.
    pub fn in_mouth(&self, pos: Vec2) -> bool {
        self.captures(pos).is_some()
    }

    /// True when the ball is fully inside the cushions.
    pub fn contains(&self, ball: &Ball) -> bool {
        let hx = self.half_length - ball.radius;
        let hy = self.half_width - ball.radius;
        ball.pos.x.abs() <= hx + 1e-9 && ball.pos.y.abs() <= hy + 1e-9
    }

    /// The foot spot, where the pack apex sits.
    pub fn foot_spot(&self) -> Vec2 {
        Vec2::new(self.half_length / 2.0, 0.0)
    }

    /// The head spot, where the cue ball starts and is re-spotted after a foul.
    pub fn head_spot(&self) -> Vec2 {
        Vec2::new(-self.half_length / 2.0, 0.0)
    }
}

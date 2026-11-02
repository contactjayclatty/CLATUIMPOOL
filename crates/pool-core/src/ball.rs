use crate::vec2::Vec2;

/// Regulation pool ball radius in metres (57.15 mm diameter).
pub const BALL_RADIUS: f64 = 0.028_575;

/// A single ball on the table.
///
/// `id` is `0` for the cue ball and `1..=15` for the object balls.
#[derive(Clone, Debug)]
pub struct Ball {
    pub id: u8,
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f64,
    /// Index into [`crate::Table::pockets`] once the ball has been potted.
    pub potted: Option<usize>,
    /// Where the ball was when it dropped, so the renderer can animate the sink.
    pub potted_at: Option<Vec2>,
    /// Follow/draw carried into the next ball contact, as a velocity boost in
    /// m/s along the original shot line. Consumed by the impact.
    pub spin: Vec2,
    /// Side spin: tangential throw applied by the next impact, in m/s.
    pub side: f64,
}

impl Ball {
    pub fn new(id: u8, pos: Vec2) -> Self {
        Self {
            id,
            pos,
            vel: Vec2::ZERO,
            radius: BALL_RADIUS,
            potted: None,
            potted_at: None,
            spin: Vec2::ZERO,
            side: 0.0,
        }
    }

    pub fn is_cue(&self) -> bool {
        self.id == 0
    }

    pub fn is_active(&self) -> bool {
        self.potted.is_none()
    }

    pub fn speed(&self) -> f64 {
        self.vel.length()
    }

    pub fn is_moving(&self) -> bool {
        self.is_active() && !self.vel.is_zero()
    }
}

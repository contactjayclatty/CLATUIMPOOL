use crate::ball::{BALL_RADIUS, Ball};
use crate::table::Table;

/// Standard eight ball pack: apex ball on the foot spot, the eight ball in the
/// middle of the third row, and one solid plus one stripe in the back corners.
const PACK: [u8; 15] = [
    1, // row 0
    9, 2, // row 1
    10, 8, 3, // row 2
    11, 4, 12, 5, // row 3
    6, 13, 7, 14, 15, // row 4
];

/// Builds the fifteen object balls plus the cue ball on its head spot.
pub fn standard_balls(table: &Table) -> Vec<Ball> {
    let mut balls = Vec::with_capacity(16);
    balls.push(Ball::new(0, table.head_spot()));

    let apex = table.foot_spot();
    let row_step = BALL_RADIUS * 3.0_f64.sqrt();
    let mut index = 0;
    for row in 0..5usize {
        for column in 0..=row {
            let x = apex.x + row as f64 * row_step;
            let y = (column as f64 - row as f64 / 2.0) * 2.0 * BALL_RADIUS;
            balls.push(Ball::new(PACK[index], crate::vec2::Vec2::new(x, y)));
            index += 1;
        }
    }

    balls
}

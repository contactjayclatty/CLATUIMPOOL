use crate::ball::Ball;
use crate::table::Table;
use crate::vec2::Vec2;

/// Tunable physics constants. Everything is in SI units (metres, seconds).
#[derive(Clone, Copy, Debug)]
pub struct PhysicsParams {
    /// Fixed simulation timestep.
    pub dt: f64,
    /// Constant deceleration while a ball rolls, in m/s².
    pub rolling_decel: f64,
    pub ball_restitution: f64,
    pub rail_restitution: f64,
    /// Fraction of the tangential speed a cushion keeps when a ball hits it.
    pub rail_friction: f64,
    /// Below this speed a ball is treated as stopped.
    pub stop_speed: f64,
    /// Upper bound on collision resolutions inside one fixed step.
    pub max_impacts_per_step: usize,
}

impl Default for PhysicsParams {
    fn default() -> Self {
        Self {
            dt: 1.0 / 240.0,
            // Cloth resistance. Real worsted cloth is close to 0.1 m/s^2, but a
            // billiard game feels dead below the value where a medium shot
            // crosses the table and comes back once.
            rolling_decel: 0.6,
            ball_restitution: 0.96,
            rail_restitution: 0.86,
            rail_friction: 0.94,
            stop_speed: 0.006,
            max_impacts_per_step: 64,
        }
    }
}

/// Which cushion a ball hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wall {
    Left,
    Right,
    Bottom,
    Top,
}

/// The two bodies involved in one collision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    /// Indices into the ball slice.
    BallBall(usize, usize),
    BallRail(usize, Wall),
}

/// One resolved collision, reported so the game layer can react to it.
#[derive(Clone, Copy, Debug)]
pub struct Impact {
    pub contact: Contact,
    /// Approach speed along the contact normal, in m/s.
    pub speed: f64,
}

/// Applies rolling friction for one fixed step.
pub fn apply_friction(balls: &mut [Ball], params: &PhysicsParams, dt: f64) {
    let drop = params.rolling_decel * dt;
    for ball in balls.iter_mut().filter(|b| b.is_active()) {
        if ball.vel.is_zero() {
            continue;
        }
        let speed = ball.speed();
        ball.vel = if speed <= drop {
            Vec2::ZERO
        } else {
            ball.vel * ((speed - drop) / speed)
        };
        if ball.speed() < params.stop_speed {
            ball.vel = Vec2::ZERO;
        }
    }
}

fn translate(balls: &mut [Ball], dt: f64) {
    for ball in balls.iter_mut().filter(|b| b.is_active()) {
        if !ball.vel.is_zero() {
            ball.pos += ball.vel * dt;
        }
    }
}

/// Swept collision time between two balls, assuming constant velocity.
fn ball_ball_toi(a: &Ball, b: &Ball) -> Option<f64> {
    let delta = b.pos - a.pos;
    let relative = b.vel - a.vel;
    let contact = a.radius + b.radius;
    let c = delta.length_squared() - contact * contact;

    if c <= 0.0 {
        // Already overlapping: resolve now only if they are closing.
        return if relative.dot(delta) < 0.0 {
            Some(0.0)
        } else {
            None
        };
    }

    let quadratic = relative.length_squared();
    if quadratic <= 1e-18 {
        return None;
    }
    let linear = 2.0 * delta.dot(relative);
    let discriminant = linear * linear - 4.0 * quadratic * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-linear - discriminant.sqrt()) / (2.0 * quadratic);
    (t >= 0.0).then_some(t)
}

/// Earliest cushion hit for one ball, assuming constant velocity.
fn rail_toi(table: &Table, ball: &Ball) -> Option<(f64, Wall)> {
    let hx = table.half_length - ball.radius;
    let hy = table.half_width - ball.radius;
    let mut best: Option<(f64, Wall)> = None;

    let mut consider = |t: f64, wall: Wall| {
        if !(t >= 0.0 && t.is_finite()) {
            return;
        }
        // The cushions are cut away at the pocket mouths, so a ball crossing
        // the cushion line inside a mouth travels on into the pocket instead
        // of bouncing off an imaginary rail.
        if table.in_mouth(ball.pos + ball.vel * t) {
            return;
        }
        let better = match best {
            Some((bt, _)) => t < bt,
            None => true,
        };
        if better {
            best = Some((t, wall));
        }
    };

    if ball.vel.x > 0.0 {
        consider((hx - ball.pos.x) / ball.vel.x, Wall::Right);
    } else if ball.vel.x < 0.0 {
        consider((-hx - ball.pos.x) / ball.vel.x, Wall::Left);
    }
    if ball.vel.y > 0.0 {
        consider((hy - ball.pos.y) / ball.vel.y, Wall::Top);
    } else if ball.vel.y < 0.0 {
        consider((-hy - ball.pos.y) / ball.vel.y, Wall::Bottom);
    }

    best
}

/// Finds the next collision within `remaining` seconds of travel.
fn earliest_impact(table: &Table, balls: &[Ball], remaining: f64) -> Option<(f64, Contact)> {
    let mut best: Option<(f64, Contact)> = None;

    for i in 0..balls.len() {
        if !balls[i].is_moving() {
            continue;
        }

        for j in (i + 1)..balls.len() {
            if !balls[j].is_active() {
                continue;
            }
            if let Some(t) = ball_ball_toi(&balls[i], &balls[j])
                && t <= remaining
                && best.is_none_or(|(bt, _)| t < bt)
            {
                best = Some((t, Contact::BallBall(i, j)));
            }
        }

        if let Some((t, wall)) = rail_toi(table, &balls[i])
            && t <= remaining
            && best.is_none_or(|(bt, _)| t < bt)
        {
            best = Some((t, Contact::BallRail(i, wall)));
        }
    }

    best
}

fn separate(a: &mut Ball, b: &mut Ball) {
    let delta = b.pos - a.pos;
    let distance = delta.length();
    let contact = a.radius + b.radius;
    if distance >= contact - 1e-9 {
        return;
    }
    let normal = if distance > 1e-12 {
        delta / distance
    } else {
        Vec2::new(1.0, 0.0)
    };
    let push = (contact - distance) * 0.5 + 1e-9;
    a.pos -= normal * push;
    b.pos += normal * push;
}

/// Removes any residual overlap left by floating point error or by a third ball
/// pushing into a resolved pair, so balls never render on top of each other.
fn resolve_overlaps(balls: &mut [Ball]) {
    for _ in 0..4 {
        let mut adjusted = false;
        for i in 0..balls.len() {
            if !balls[i].is_active() {
                continue;
            }
            for j in (i + 1)..balls.len() {
                if !balls[j].is_active() {
                    continue;
                }
                let delta = balls[j].pos - balls[i].pos;
                let contact = balls[i].radius + balls[j].radius;
                if delta.length_squared() >= (contact - 1e-9) * (contact - 1e-9) {
                    continue;
                }
                let (left, right) = balls.split_at_mut(j);
                separate(&mut left[i], &mut right[0]);
                adjusted = true;
            }
        }
        if !adjusted {
            break;
        }
    }
}

fn resolve(table: &Table, balls: &mut [Ball], params: &PhysicsParams, contact: Contact) -> f64 {
    match contact {
        Contact::BallBall(i, j) => {
            let (left, right) = balls.split_at_mut(j);
            let a = &mut left[i];
            let b = &mut right[0];

            let delta = b.pos - a.pos;
            let normal = if delta.length() > 1e-12 {
                delta.normalized()
            } else {
                Vec2::new(1.0, 0.0)
            };
            let closing = (b.vel - a.vel).dot(normal);
            let speed = (-closing).max(0.0);

            if closing < 0.0 {
                let impulse = -(1.0 + params.ball_restitution) * closing * 0.5;
                a.vel -= normal * impulse;
                b.vel += normal * impulse;
            }

            // Side spin throws the object ball off the line of centres and
            // drags the cue the other way.
            let side = a.side + b.side;
            if side != 0.0 {
                let throw_dir = Vec2::new(-normal.y, normal.x) * (side * 0.22);
                a.vel -= throw_dir;
                b.vel += throw_dir;
                a.side = 0.0;
                b.side = 0.0;
            }

            // Follow and draw bite when the cue ball strikes an object ball.
            for ball in [&mut *a, &mut *b] {
                if !ball.spin.is_zero() {
                    ball.vel += ball.spin;
                    ball.spin = Vec2::ZERO;
                }
            }

            separate(a, b);
            speed
        }
        Contact::BallRail(index, wall) => {
            let ball = &mut balls[index];
            let hx = table.half_length - ball.radius;
            let hy = table.half_width - ball.radius;
            let horizontal = matches!(wall, Wall::Left | Wall::Right);
            let limit = if horizontal { hx } else { hy };
            let inward = match wall {
                Wall::Left | Wall::Bottom => 1.0,
                Wall::Right | Wall::Top => -1.0,
            };
            let sign = -inward;

            let (axis_pos, axis_vel) = if horizontal {
                (&mut ball.pos.x, &mut ball.vel.x)
            } else {
                (&mut ball.pos.y, &mut ball.vel.y)
            };
            let speed = axis_vel.abs();
            if *axis_vel * inward < 0.0 {
                *axis_vel = -*axis_vel * params.rail_restitution;
            }
            *axis_pos = sign * (limit - 1e-9);

            // A real cushion also grabs the ball travelling along it.
            let tangent = if horizontal {
                &mut ball.vel.y
            } else {
                &mut ball.vel.x
            };
            *tangent *= params.rail_friction;
            *tangent += ball.side * 0.45;
            ball.side *= 0.55;
            speed
        }
    }
}

/// Last line of defence: a ball squeezed out of the cushions by an overlap
/// correction or by floating point drift is slid back onto the cloth. A ball
/// inside a pocket mouth is left alone, because it is falling in rather than
/// resting on the rail.
fn contain(table: &Table, balls: &mut [Ball]) {
    for ball in balls.iter_mut().filter(|ball| ball.is_active()) {
        if table.in_mouth(ball.pos) {
            continue;
        }
        let hx = table.half_length - ball.radius;
        let hy = table.half_width - ball.radius;
        if ball.pos.x > hx {
            ball.pos.x = hx;
            ball.vel.x = ball.vel.x.min(0.0);
        } else if ball.pos.x < -hx {
            ball.pos.x = -hx;
            ball.vel.x = ball.vel.x.max(0.0);
        }
        if ball.pos.y > hy {
            ball.pos.y = hy;
            ball.vel.y = ball.vel.y.min(0.0);
        } else if ball.pos.y < -hy {
            ball.pos.y = -hy;
            ball.vel.y = ball.vel.y.max(0.0);
        }
    }
}

/// Advances every active ball by one fixed step, resolving collisions by
/// time of impact rather than by discrete overlap tests.
pub fn advance(table: &Table, balls: &mut [Ball], params: &PhysicsParams, dt: f64) -> Vec<Impact> {
    let mut impacts = Vec::new();
    let mut remaining = dt;
    let mut guard = 0;

    while remaining > 1e-12 && guard < params.max_impacts_per_step {
        guard += 1;
        match earliest_impact(table, balls, remaining) {
            Some((t, contact)) => {
                if t > 0.0 {
                    translate(balls, t);
                    remaining -= t;
                } else {
                    remaining = remaining.max(0.0);
                }
                let speed = resolve(table, balls, params, contact);
                impacts.push(Impact { contact, speed });
            }
            None => {
                translate(balls, remaining);
                remaining = 0.0;
            }
        }
    }

    resolve_overlaps(balls);
    contain(table, balls);
    impacts
}

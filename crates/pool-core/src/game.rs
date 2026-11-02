use crate::ball::{BALL_RADIUS, Ball};
use crate::physics::{self, Contact, PhysicsParams};
use crate::rack;
use crate::rules::{Group, Rules, ShotOutcome, ShotReport};
use crate::table::Table;
use crate::vec2::Vec2;

/// Fastest cue ball speed reachable with full power, in m/s.
pub const MAX_SHOT_SPEED: f64 = 6.5;

/// Things the presentation layer may want to react to each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    BallHit { a: u8, b: u8, speed: f64 },
    RailHit { ball: u8, speed: f64 },
    Potted { ball: u8, pocket: usize },
    ShotStarted,
    ShotResolved { outcome: ShotOutcome },
    Rest,
    GameOver { winner: usize },
}

#[derive(Clone, Debug, Default)]
struct ShotState {
    first_contact: Option<u8>,
    rail_after_contact: bool,
    potted: Vec<u8>,
    cue_potted: bool,
}

/// The whole simulation: table, balls, physics settings and rules.
///
/// [`Game::update`] is driven with wall-clock frame deltas and internally steps
/// the simulation at a fixed timestep, so results do not depend on frame rate.
#[derive(Clone, Debug)]
pub struct Game {
    pub table: Table,
    pub balls: Vec<Ball>,
    pub params: PhysicsParams,
    pub rules: Rules,
    pub shots_taken: u32,
    events: Vec<Event>,
    accumulator: f64,
    shot: Option<ShotState>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        let table = Table::standard();
        let balls = rack::standard_balls(&table);
        Self {
            table,
            balls,
            params: PhysicsParams::default(),
            rules: Rules::new(),
            shots_taken: 0,
            events: Vec::new(),
            accumulator: 0.0,
            shot: None,
        }
    }

    /// Resets the table to a fresh rack with player one to break.
    pub fn rack(&mut self) {
        self.balls = rack::standard_balls(&self.table);
        self.rules = Rules::new();
        self.shot = None;
        self.accumulator = 0.0;
        self.shots_taken = 0;
        self.events.push(Event::Rest);
    }

    pub fn cue_ball(&self) -> &Ball {
        &self.balls[0]
    }

    pub fn all_at_rest(&self) -> bool {
        self.balls.iter().all(|ball| !ball.is_moving())
    }

    /// True when a new shot may be taken.
    pub fn ready(&self) -> bool {
        self.all_at_rest() && self.shot.is_none() && self.rules.winner.is_none()
    }

    /// Fires the cue ball along `direction`; `power` is clamped to `0..=1`.
    pub fn shoot(&mut self, direction: Vec2, power: f64) -> bool {
        self.shoot_spin(direction, power, 0.0, 0.0)
    }

    /// Fires the cue ball with follow (`-1` draw … `1` follow) and side spin
    /// (`-1` left … `1` right). Both offsets are relative to the shot line.
    pub fn shoot_spin(&mut self, direction: Vec2, power: f64, follow: f64, side: f64) -> bool {
        if !self.ready() {
            return false;
        }
        let direction = direction.normalized();
        if direction.is_zero() {
            return false;
        }
        let speed = power.clamp(0.0, 1.0) * MAX_SHOT_SPEED;
        if speed <= 0.0 {
            return false;
        }

        for ball in &mut self.balls {
            ball.spin = Vec2::ZERO;
            ball.side = 0.0;
        }
        let cue = &mut self.balls[0];
        cue.vel = direction * speed;
        cue.spin = direction * (follow.clamp(-1.0, 1.0) * speed * 0.5);
        cue.side = side.clamp(-1.0, 1.0) * speed * 0.18;

        self.shot = Some(ShotState::default());
        self.shots_taken += 1;
        self.rules.ball_in_hand = false;
        self.events.push(Event::ShotStarted);
        true
    }

    /// Moves the cue ball to `pos` when ball-in-hand, if the spot is legal.
    pub fn place_cue_ball(&mut self, pos: Vec2) -> bool {
        if !self.ready() || !self.rules.ball_in_hand {
            return false;
        }
        if !self.spot_is_free(pos) {
            return false;
        }
        let cue = &mut self.balls[0];
        cue.pos = pos;
        cue.vel = Vec2::ZERO;
        cue.spin = Vec2::ZERO;
        cue.side = 0.0;
        cue.potted = None;
        cue.potted_at = None;
        self.rules.ball_in_hand = false;
        true
    }

    /// True when a cue ball could be placed at `pos` without touching anything.
    pub fn spot_is_free(&self, pos: Vec2) -> bool {
        let radius = self.balls[0].radius;
        let hx = self.table.half_length - radius;
        let hy = self.table.half_width - radius;
        if pos.x.abs() > hx || pos.y.abs() > hy {
            return false;
        }
        // Never let a player park the cue ball inside a pocket mouth.
        if self.table.in_mouth(pos) {
            return false;
        }
        self.balls
            .iter()
            .skip(1)
            .filter(|ball| ball.is_active())
            .all(|ball| ball.pos.distance(pos) > 2.0 * radius + 1e-6)
    }

    /// Advances the simulation by one frame worth of time.
    pub fn update(&mut self, frame_dt: f64) {
        if self.all_at_rest() {
            self.accumulator = 0.0;
            return;
        }

        self.accumulator += frame_dt.clamp(0.0, 0.25);
        let dt = self.params.dt;
        let mut steps = 0;
        while self.accumulator >= dt && steps < 64 {
            self.fixed_step(dt);
            self.accumulator -= dt;
            steps += 1;
            if self.all_at_rest() {
                self.accumulator = 0.0;
                break;
            }
        }
    }

    fn fixed_step(&mut self, dt: f64) {
        physics::apply_friction(&mut self.balls, &self.params, dt);
        let impacts = physics::advance(&self.table, &mut self.balls, &self.params, dt);
        for impact in impacts {
            self.record_impact(impact);
        }
        self.capture_pockets();

        if self.all_at_rest() && self.shot.is_some() {
            self.finish_shot();
        }
    }

    fn record_impact(&mut self, impact: physics::Impact) {
        match impact.contact {
            Contact::BallBall(i, j) => {
                let (a, b) = (self.balls[i].id, self.balls[j].id);
                self.events.push(Event::BallHit {
                    a,
                    b,
                    speed: impact.speed,
                });
                if let Some(state) = self.shot.as_mut()
                    && state.first_contact.is_none()
                {
                    if a == 0 {
                        state.first_contact = Some(b);
                    } else if b == 0 {
                        state.first_contact = Some(a);
                    }
                }
            }
            Contact::BallRail(i, _wall) => {
                let ball = self.balls[i].id;
                self.events.push(Event::RailHit {
                    ball,
                    speed: impact.speed,
                });
                if let Some(state) = self.shot.as_mut()
                    && state.first_contact.is_some()
                {
                    state.rail_after_contact = true;
                }
            }
        }
    }

    fn capture_pockets(&mut self) {
        for index in 0..self.balls.len() {
            if !self.balls[index].is_active() {
                continue;
            }
            let pos = self.balls[index].pos;
            let pocket = self.table.captures(pos).or_else(|| {
                // The cushions are cut away at the pocket mouths, so anything
                // that crossed the cushion line has already gone in.
                let hx = self.table.half_length - self.balls[index].radius + 1e-6;
                let hy = self.table.half_width - self.balls[index].radius + 1e-6;
                if pos.x.abs() <= hx && pos.y.abs() <= hy {
                    return None;
                }
                self.table
                    .pockets
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.center.distance(pos).total_cmp(&b.center.distance(pos))
                    })
                    .map(|(index, _)| index)
            });
            let Some(pocket) = pocket else {
                continue;
            };

            let ball = &mut self.balls[index];
            let dropped_from = ball.pos;
            ball.potted = Some(pocket);
            ball.potted_at = Some(dropped_from);
            ball.vel = Vec2::ZERO;
            let id = ball.id;

            self.events.push(Event::Potted { ball: id, pocket });
            if let Some(state) = self.shot.as_mut() {
                if id == 0 {
                    state.cue_potted = true;
                } else {
                    state.potted.push(id);
                }
            }
        }
    }

    fn finish_shot(&mut self) {
        let state = self.shot.take().unwrap_or_default();
        let shooter = self.rules.current_player;
        let report = ShotReport {
            first_contact: state.first_contact,
            rail_after_contact: state.rail_after_contact,
            potted: state.potted.clone(),
            cue_potted: state.cue_potted,
            shooter_cleared: self.group_cleared(shooter),
            on_break: self.shots_taken <= 1,
        };

        let outcome = self.rules.resolve_shot(&report);
        self.events.push(Event::ShotResolved { outcome });

        // An eight ball that drops on the break goes back on the foot spot.
        if state.potted.contains(&8) && self.rules.winner.is_none() {
            self.respot_eight();
        }

        if state.cue_potted && self.rules.winner.is_none() {
            self.respot_cue_ball();
        }

        self.events.push(Event::Rest);

        if let Some(winner) = self.rules.winner {
            self.events.push(Event::GameOver { winner });
        }
    }

    fn group_cleared(&self, player: usize) -> bool {
        let Some(group) = self.rules.groups[player] else {
            return false;
        };
        !self
            .balls
            .iter()
            .skip(1)
            .any(|ball| ball.is_active() && crate::rules::group_of(ball.id) == Some(group))
    }

    fn respot_cue_ball(&mut self) {
        let head = self.table.head_spot();
        let mut spot = head;
        let mut offset = 0.0;
        while !self.spot_is_free(spot) && offset < self.table.half_length {
            offset += 0.01;
            spot = Vec2::new(head.x + offset, head.y);
        }

        let cue = &mut self.balls[0];
        cue.pos = spot;
        cue.vel = Vec2::ZERO;
        cue.spin = Vec2::ZERO;
        cue.side = 0.0;
        cue.potted = None;
        cue.potted_at = None;
        self.rules.ball_in_hand = true;
        self.events.push(Event::Rest);
    }

    /// Returns the eight ball to the foot spot after it drops on the break.
    fn respot_eight(&mut self) {
        let foot = self.table.foot_spot();
        let occupied: Vec<Vec2> = self
            .balls
            .iter()
            .filter(|ball| ball.id != 8 && ball.is_active())
            .map(|ball| ball.pos)
            .collect();

        let mut spot = foot;
        let mut offset = 0.0;
        while occupied
            .iter()
            .any(|pos| pos.distance(spot) <= 2.0 * BALL_RADIUS + 1e-6)
            && offset < self.table.half_length
        {
            offset += 0.005;
            spot = Vec2::new(foot.x - offset, foot.y);
        }

        if let Some(ball) = self.balls.iter_mut().find(|ball| ball.id == 8) {
            ball.pos = spot;
            ball.vel = Vec2::ZERO;
            ball.spin = Vec2::ZERO;
            ball.side = 0.0;
            ball.potted = None;
            ball.potted_at = None;
        }
        self.events.push(Event::Rest);
    }

    /// Remaining balls of a player's group that are still on the table.
    pub fn remaining_in_group(&self, group: Group) -> usize {
        self.balls
            .iter()
            .skip(1)
            .filter(|ball| ball.is_active() && crate::rules::group_of(ball.id) == Some(group))
            .count()
    }

    pub fn eight_ball_on_table(&self) -> bool {
        self.balls
            .iter()
            .any(|ball| ball.id == 8 && ball.is_active())
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

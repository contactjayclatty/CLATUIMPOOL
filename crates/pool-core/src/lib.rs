//! Deterministic 2D billiards simulation used by CLATUIMPOOL.
//!
//! The core knows nothing about rendering, audio or input: it exposes a table,
//! a set of balls, a fixed-step physics loop and simplified eight ball rules.
//! Everything is in SI units, with the table centred on the origin and the
//! `y` axis pointing up. Balls are `0.028575` m in radius unless overridden.

pub mod ball;
pub mod game;
pub mod physics;
pub mod rack;
pub mod rules;
pub mod table;
pub mod vec2;

pub use ball::{BALL_RADIUS, Ball};
pub use game::{Event, Game, MAX_SHOT_SPEED};
pub use physics::{Contact, Impact, PhysicsParams, Wall};
pub use rules::{Foul, Group, Rules, ShotOutcome, ShotReport, group_of};
pub use table::{PLAY_LENGTH, PLAY_WIDTH, Pocket, Table};
pub use vec2::Vec2;

#[cfg(test)]
mod tests {
    use super::*;

    fn run_until_rest(game: &mut Game, limit_seconds: f64) -> f64 {
        let mut elapsed = 0.0;
        while !game.all_at_rest() && elapsed < limit_seconds {
            game.update(1.0 / 60.0);
            elapsed += 1.0 / 60.0;
        }
        elapsed
    }

    #[test]
    fn fresh_rack_is_legal() {
        let game = Game::new();
        assert_eq!(game.balls.len(), 16);
        let table = &game.table;
        for ball in &game.balls {
            assert!(table.contains(ball), "ball {} is off the table", ball.id);
            assert!(ball.is_active());
        }
        assert!(game.all_at_rest());
        assert!(game.ready());
        assert_eq!(game.rules.current_player, 0);
    }

    #[test]
    fn balls_never_leave_the_table_and_energy_never_grows() {
        let mut game = Game::new();
        // Hard break straight into the pack.
        assert!(game.shoot(Vec2::new(1.0, 0.02), 1.0));

        let mut previous_energy = total_energy(&game);
        while !game.all_at_rest() {
            game.update(1.0 / 60.0);
            let energy = total_energy(&game);
            assert!(
                energy <= previous_energy + 1e-6,
                "energy grew from {previous_energy} to {energy}"
            );
            previous_energy = energy;
            for ball in &game.balls {
                if ball.is_active() {
                    assert!(
                        game.table.contains(ball),
                        "ball {} escaped to {:?}",
                        ball.id,
                        ball.pos
                    );
                }
            }
        }
    }

    #[test]
    fn straight_shot_into_a_corner_pocket_pots_the_cue_ball() {
        let mut game = Game::new();
        let pocket = game.table.pockets[3].center;
        let start = crate::vec2::Vec2::new(pocket.x - 0.60, pocket.y - 0.30);
        game.balls[0].pos = start;
        let direction = pocket - start;

        assert!(game.shoot(direction, 0.35));
        run_until_rest(&mut game, 20.0);

        assert_eq!(
            game.rules.last_outcome,
            Some(ShotOutcome::Foul(Foul::CueBallPotted)),
            "potting the cue ball must be a foul"
        );
        // The cue ball is re-spotted for the opponent, who has ball in hand.
        assert!(game.balls[0].is_active());
        assert!(game.rules.ball_in_hand);
        assert_eq!(game.rules.current_player, 1);
    }

    #[test]
    fn a_ball_at_rest_stays_at_rest() {
        let game = Game::new();
        let snapshot = game.balls.iter().map(|b| b.pos).collect::<Vec<_>>();
        let mut game = game;
        for _ in 0..120 {
            game.update(1.0 / 60.0);
        }
        for (ball, before) in game.balls.iter().zip(snapshot) {
            assert_eq!(ball.pos, before);
        }
    }

    #[test]
    fn shot_outcome_is_repeatable_for_the_same_input() {
        let mut a = Game::new();
        let mut b = Game::new();
        a.shoot(Vec2::new(1.0, 0.01), 0.8);
        b.shoot(Vec2::new(1.0, 0.01), 0.8);
        run_until_rest(&mut a, 40.0);
        run_until_rest(&mut b, 40.0);
        assert_eq!(a.balls.len(), b.balls.len());
        for (left, right) in a.balls.iter().zip(&b.balls) {
            assert_eq!(left.id, right.id);
            assert_eq!(left.potted, right.potted);
            assert!((left.pos.x - right.pos.x).abs() < 1e-12);
            assert!((left.pos.y - right.pos.y).abs() < 1e-12);
        }
    }

    #[test]
    fn break_contact_hands_the_turn_over_or_keeps_it() {
        let mut game = Game::new();
        game.shoot(Vec2::new(1.0, 0.0), 0.9);
        run_until_rest(&mut game, 40.0);
        assert!(game.rules.last_outcome.is_some());
    }

    #[test]
    fn cue_ball_transfers_momentum_into_an_object_ball() {
        let mut game = Game::new();
        // Leave only the cue ball and ball 1 on the table, lined up on the x axis.
        for ball in game.balls.iter_mut().skip(2) {
            ball.potted = Some(0);
        }
        game.balls[1].pos = crate::vec2::Vec2::new(0.0, 0.0);
        game.balls[0].pos = crate::vec2::Vec2::new(-0.40, 0.0);

        assert!(game.shoot(Vec2::new(1.0, 0.0), 0.30));

        let start = game.balls[1].pos;
        let mut elapsed = 0.0;
        while elapsed < 3.0 && !game.all_at_rest() {
            game.update(1.0 / 60.0);
            elapsed += 1.0 / 60.0;
        }

        assert!(
            game.balls[1].pos.distance(start) > 0.1,
            "the object ball never moved, balls are not colliding"
        );
        assert!(
            game.balls[0].speed() < 0.5,
            "the cue ball should have given up most of its speed"
        );
    }

    fn report(first: Option<u8>, potted: Vec<u8>) -> ShotReport {
        ShotReport {
            first_contact: first,
            rail_after_contact: true,
            potted,
            cue_potted: false,
            shooter_cleared: false,
            on_break: false,
        }
    }

    #[test]
    fn potting_your_own_ball_keeps_the_turn_and_fills_the_tray() {
        let mut rules = Rules::new();
        let outcome = rules.resolve_shot(&report(Some(1), vec![2, 1]));
        assert_eq!(outcome, ShotOutcome::Continue);
        assert_eq!(rules.groups[0], Some(Group::Solids));
        assert_eq!(rules.groups[1], Some(Group::Stripes));
        assert_eq!(rules.potted_by(0), &[1, 2][..]);
        assert_eq!(rules.potted_by(1), &[][..]);
        assert_eq!(rules.current_player, 0);
    }

    #[test]
    fn potting_only_the_opponents_ball_turns_the_table_over() {
        let mut rules = Rules::new();
        rules.groups = [Some(Group::Solids), Some(Group::Stripes)];
        let outcome = rules.resolve_shot(&report(Some(1), vec![9]));
        assert_eq!(outcome, ShotOutcome::TurnOver);
        assert_eq!(rules.current_player, 1);
        assert_eq!(rules.potted_by(0), &[9][..]);
    }

    #[test]
    fn eight_on_the_break_is_not_a_result() {
        let mut rules = Rules::new();
        let outcome = rules.resolve_shot(&ShotReport {
            on_break: true,
            ..report(Some(1), vec![8])
        });
        assert!(
            rules.winner.is_none(),
            "a break pot of the eight is re-spotted"
        );
        assert_eq!(outcome, ShotOutcome::TurnOver);
    }

    #[test]
    fn striking_the_eight_first_on_an_open_table_is_a_foul() {
        let mut rules = Rules::new();
        let outcome = rules.resolve_shot(&report(Some(8), vec![]));
        assert_eq!(outcome, ShotOutcome::Foul(Foul::WrongBallFirst));
        assert!(rules.ball_in_hand);
        assert_eq!(rules.current_player, 1);
    }

    #[test]
    fn follow_spin_drags_the_cue_ball_forward() {
        let build = || {
            let mut game = Game::new();
            for ball in game.balls.iter_mut().skip(2) {
                ball.potted = Some(0);
            }
            game.balls[1].pos = Vec2::new(0.0, 0.0);
            game.balls[0].pos = Vec2::new(-0.40, 0.0);
            game
        };
        let mut plain = build();
        let mut follow = build();
        plain.shoot(Vec2::new(1.0, 0.0), 0.45);
        follow.shoot_spin(Vec2::new(1.0, 0.0), 0.45, 1.0, 0.0);
        run_until_rest(&mut plain, 15.0);
        run_until_rest(&mut follow, 15.0);
        assert!(
            follow.balls[0].pos.x > plain.balls[0].pos.x + 0.05,
            "follow spin should carry the cue ball past a plain stun shot"
        );
    }

    #[test]
    fn balls_never_rest_outside_the_cushions() {
        for step in 0..48 {
            let angle = step as f64 / 48.0 * std::f64::consts::TAU;
            let direction = Vec2::new(angle.cos(), angle.sin() * 0.6);
            for power in [0.25, 0.6, 1.0] {
                let mut game = Game::new();
                game.shoot(direction, power);
                let mut elapsed = 0.0;
                while elapsed < 30.0 && !game.all_at_rest() {
                    game.update(1.0 / 60.0);
                    elapsed += 1.0 / 60.0;
                    for ball in game.balls.iter().filter(|b| b.is_active()) {
                        assert!(
                            game.table.contains(ball),
                            "ball {} left the table at {:?} (dir {angle:.3}, power {power})",
                            ball.id,
                            ball.pos
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_lag_shot_rolls_the_length_of_the_table() {
        // Guards the cloth resistance: too much friction and billiards feels
        // dead, because a gentle shot dies before it reaches the far cushion.
        let mut game = Game::new();
        for ball in game.balls.iter_mut().skip(1) {
            ball.potted = Some(0);
        }
        game.balls[0].pos = Vec2::new(-1.2, 0.0);
        let power = 1.8 / crate::game::MAX_SHOT_SPEED;
        assert!(game.shoot(Vec2::new(1.0, 0.0), power));
        run_until_rest(&mut game, 30.0);
        assert!(
            game.balls[0].pos.x > 0.8,
            "the cue ball stalled at x = {}, it should have crossed the table",
            game.balls[0].pos.x
        );
    }

    fn total_energy(game: &Game) -> f64 {
        game.balls
            .iter()
            .filter(|ball| ball.is_active())
            .map(|ball| ball.vel.length_squared())
            .sum()
    }
}

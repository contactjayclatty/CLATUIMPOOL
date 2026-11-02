# CLATUIMPOOL - project brainstorm

Status: seeded 2026-11-02. Direction chosen: **game / simulation**. Nothing built yet
beyond the `cargo new` scaffold (`src/main.rs` prints `Hello, world!`).

## 1. Reading the name

"CLATUIMPOOL" reads like *CLATIUM + POOL*. Two useful readings:

- **billiards / pool** - the table game: balls, cue, rails, pockets, physics.
- **resource pool** - already ruled out, this is a game/simulation project.

The billiards reading is the strongest: it is a simulation first and a game second,
which fits a Rust crate better than an art-heavy game would.

## 2. Direction options

| Option | What it is | Effort | Risk |
| --- | --- | --- | --- |
| **A: 2D pool / billiards sim** (recommended) | Deterministic 2D ball physics on a table, cue aiming, pockets, shot playback | Low-medium | Low - well understood physics, no asset pipeline |
| B: pool-hall tycoon / ecosystem sim | Agents (players, tables, customers) with schedules and economy on top of the physics | Medium-high | Scope creep; needs the physics core anyway |
| C: abstract "pool" sandbox | Particles or fluid in a bounded arena, emergent behaviour, no scoring | Medium | Visual payoff needs a renderer early; harder to define "done" |

A is the recommendation because it gives a testable core (physics + rules) that B can
later sit on. B without A is a spreadsheet with no soul; C is a different project.

## 3. Scope boundaries

- In scope: 2D top-down billiards, single table, standard 8-ball-ish rules, single
  player vs simple bot or solo practice.
- Out of scope for now: 3D, real cue-ball spin models beyond basic English, networking,
  mobile, art assets, monetisation, level editor.
- Must not become: a general-purpose physics engine, or a clone of an existing
  commercial pool game with its asset set.

## 4. MVP cut (v0)

First runnable version:

1. Headless simulation: table rectangle, pockets, N balls, constant-friction motion,
   ball-ball and ball-rail elastic collisions with restitution, shot resolved to rest.
2. Deterministic: fixed timestep, seeded RNG, same input -> same final layout.
3. Text/ASCII debug render of ball positions, plus a shot log.
4. Unit tests: energy never increases, balls never leave the table, a straight shot
   into a pocket drops the ball.

Not in v0: aiming UI, sound, textures, AI opponent, rule enforcement beyond potting,
spin/English, jump shots.

Definition of done for v0: `cargo run -- --shot "30,0.8"` prints a stable, repeatable
outcome for a canned table layout, and the test suite covers collision invariants.

## 5. Technical notes

- Crate type: binary now; split into `pool-core` (sim) + `pool-cli` later if a GUI arrives.
- Physics: fixed-step integrator (e.g. 240 Hz), sub-stepped collision resolution,
  position correction to avoid tunnelling. Use `f64` internally, not `f32`.
- Rendering later: keep the core free of any graphics dependency so the choice
  (`macroquad`, `bevy`, `wgpu`) stays open. Renderer should be a thin viewer over the
  core, never the source of truth.
- Dependencies: start with none. Add only what the MVP cannot reasonably do itself.
- Platform: Windows first; core stays platform-agnostic.
- Testing: unit tests on the core, plus a golden-file test of a canned shot sequence.
- Determinism rule for the future: no floating-point order-dependent reductions across
  threads in the core.

## 6. Risks and unknowns

- **Tunnelling** at high cue speed: needs sub-stepping or swept collision tests. Spike this first.
- **Rest detection**: friction + sleep thresholds can chatter; needs a deliberate rule.
- **Pocket geometry**: a real pocket is a jaw shape, not a circle; a circle-only model may
  feel wrong. Decide how faithful v0 must be.
- **Feel**: accurate physics can still feel bad; tuning is iteration-heavy and cannot be
  validated by tests alone.

## 7. Open questions

- [ ] Realistic table dimensions and ball radii (regulation vs game-feel)?
- [ ] Full rules engine in v0 or potting only?
- [ ] Input model: numeric shot specification for v0, mouse drag later?
- [ ] Bot opponent eventually, or solo practice only?

## 8. Next actions

- [ ] Confirm direction A (2D billiards sim) over B and C
- [ ] Spike swept-collision tunnelling test at max cue speed
- [ ] Write `pool-core` module skeleton: `Table`, `Ball`, `Shot`, `step()`
- [ ] Replace `Hello, world!` with a headless demo that prints a shot outcome

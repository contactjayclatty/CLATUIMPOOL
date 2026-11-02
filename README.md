# CLATUIMPOOL

A 2D eight ball game in Rust. Deterministic simulation core, thin macroquad renderer.

```
crates/pool-core     physics + rules, no graphics dependencies
crates/clatuimpool   window, screens, input and drawing
```

## Download

Grab `clatuimpool-v<version>-win64.zip` from the
[releases page](../../releases/latest), unpack it and run `clatuimpool.exe`.
It is a single self-contained binary: no installer, no runtime to install, no
console window. Windows 10 or newer, 64-bit.

## Build from source

```
cargo run --release
```

Requires a Rust toolchain with edition 2024 support (1.85 or newer). The Windows build is
GUI-only: no console window appears, whether you launch it from a terminal, a shortcut or
Explorer. While debugging you can bring the terminal back with
`cargo run --features console`.

First build pulls macroquad and takes a couple of minutes; after that it starts instantly.
Debug builds work but the release profile is noticeably smoother.

Settings are written to `clatuimpool.cfg` in the working directory. Delete it to reset
everything to defaults.

## Controls

| Input | Action |
| --- | --- |
| Move mouse | Aim: the cue stick sits between the cursor and the cue ball |
| Hold left button and drag away from the cue ball | Pull the cue back; further = harder |
| Release left button | Shoot |
| Drag inside the spin pad | Set follow / draw (up, down) and side spin (left, right) |
| Left click | Place the cue ball when the banner says BALL IN HAND |
| `R` | New rack |
| `Esc` | Pause menu (resume, settings, new rack, main menu) |
| `Enter` | Play again on the game over screen |

## Screens

- **Menu** - play, settings, quit.
- **Table** - two player cards with group and balls left, a potted-ball tray for the
  player whose turn it is, shot commentary, a power meter while you pull back, and the
  cue ball spin pad.
- **Settings** - aim guide, predicted ball lines, ghost ball, spin pad, potted tray,
  shot commentary, screen shake, slow motion, fullscreen, sink speed, pull distance and
  four table finishes. Everything persists between runs.
- **Pause / Game over** - resume, new rack, main menu; winner, shots and balls potted.

## What works

- Six pockets, fifteen object balls, cue ball, regulation proportions (2.54 m x 1.27 m).
- Standard colours and numbering: white cue ball, yellow 1, blue 2, red 3, purple 4,
  orange 5, green 6, maroon 7, black 8, and 9-15 as striped versions of 1-7.
- Standard rack: 1 on the foot spot, 8 in the middle of the third row, one solid and
  one stripe in the back corners.
- **Cue ball spin**: follow drags the cue ball forward after contact, draw pulls it back,
  side spin throws the object ball off the line of centres and kicks off the cushions.
- **Potting animation**: the ball slides the last centimetres into the pocket with an
  accelerating ease, sinks below the rim, fades out, and ripples the cloth.
- **Potted tray**: each player's sunk balls are listed on their card and, larger, for the
  player currently at the table. It switches with the turn.
- Aiming aid: dashed cue path to first contact, a ghost ball at the contact point, the
  object ball's predicted line (yellow, with arrow head) and the cue ball's deflection
  (pale blue). Every part can be switched off in settings.
- Pocket mouths are cut into the cushions: a ball crossing the cushion line inside a
  mouth drops in instead of bouncing off an invisible rail, and a ball is potted exactly
  when its centre enters the drawn hole, so nothing can rest on a pocket rim.
- Balls cannot rest on a cushion either: every step ends with a containment pass that
  slides a ball squeezed out by an overlap back onto the cloth, and ball-in-hand
  placement rejects the pocket mouths.
- Cushion cloth grabs a little of the ball's tangential speed, the cloth rolls true
  (a lag shot crosses the table and comes back), breaks run up to 6.5 m/s, and every
  step ends with an overlap clean-up so balls never render inside each other.
- Ball in hand: a ghost cue ball follows the cursor, green where it can be placed and
  red against a ball, a cushion or a pocket mouth. Soft shots get finer power control
  than a straight linear pull.
- Continuous collision detection: swept time-of-impact for ball-ball and ball-cushion,
  so fast shots do not tunnel through other balls.
- Rolling friction, restitution on cushions and between balls, rest detection.
- Eight ball rules: open table, group assignment, turn passing, fouls for potting the
  cue ball / hitting nothing / wrong group first / hitting the eight first / no cushion
  after contact, potting the eight early loses, potting it with a clean group clear
  wins, the eight is re-spotted when it drops on the break, and the cue ball comes back
  with ball in hand after a foul.

## Known simplifications

- Spin is a velocity transfer on contact, not an angular-momentum solve; there are no
  masse shots and no jump shots.
- Spin is not drawn on the ball itself, only on the pad.
- Potting your last group ball and the eight ball in the same shot counts as a win.
- No safeties and no call-shot rules.
- Two players share one screen; there is no opponent AI.

## Checks

```
cargo test -p pool-core     # 14 simulation and rules tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

The core tests cover table containment, non-increasing kinetic energy, a straight shot
into a corner pocket, rest stability, rack legality, shot determinism, momentum transfer,
follow spin, a lag shot rolling the length of the table, tray bookkeeping and the
eight-ball edge cases.

## Releasing

The version lives in one place: `version` under `[workspace.package]` in the root
`Cargo.toml`. Both crates inherit it.

```
pwsh -File scripts/package.ps1
```

builds `target\release\clatuimpool.exe` and zips it with the README and both licence
files into `dist\clatuimpool-v<version>-win64.zip`.

Publishing is automated: `.github/workflows/release.yml` runs fmt, clippy and the tests on
`windows-latest`, packages the zip and attaches it to a GitHub release when a `v*` tag is
pushed.

```
git tag v0.1.0
git push origin v0.1.0
```

## Licence

MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.

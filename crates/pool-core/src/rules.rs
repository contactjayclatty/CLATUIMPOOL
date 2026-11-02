/// Which set of object balls a player owns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Solids,
    Stripes,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Self::Solids => "solids",
            Self::Stripes => "stripes",
        }
    }
}

/// The group a ball belongs to, or `None` for the cue ball and the eight ball.
pub fn group_of(id: u8) -> Option<Group> {
    match id {
        1..=7 => Some(Group::Solids),
        9..=15 => Some(Group::Stripes),
        _ => None,
    }
}

/// Why a shot was a foul.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foul {
    CueBallPotted,
    NoBallContact,
    WrongBallFirst,
    NoRailContact,
}

impl Foul {
    pub fn label(self) -> &'static str {
        match self {
            Self::CueBallPotted => "cue ball potted",
            Self::NoBallContact => "no ball hit",
            Self::WrongBallFirst => "wrong ball hit first",
            Self::NoRailContact => "no cushion after contact",
        }
    }
}

/// What happened over the course of one shot once everything came to rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotOutcome {
    /// Same player shoots again.
    Continue,
    /// Legal shot with nothing potted: play passes over.
    TurnOver,
    Foul(Foul),
    Win {
        player: usize,
    },
    Lose {
        player: usize,
    },
}

/// Everything the rules need to know about a completed shot.
#[derive(Clone, Debug, Default)]
pub struct ShotReport {
    pub first_contact: Option<u8>,
    pub rail_after_contact: bool,
    pub potted: Vec<u8>,
    pub cue_potted: bool,
    /// True when the shooter's own group was already cleared before this shot.
    pub shooter_cleared: bool,
    /// True when this was the opening break shot.
    pub on_break: bool,
}

/// Two-player eight ball rules for hot-seat play.
#[derive(Clone, Debug)]
pub struct Rules {
    pub current_player: usize,
    pub groups: [Option<Group>; 2],
    pub winner: Option<usize>,
    pub ball_in_hand: bool,
    pub last_outcome: Option<ShotOutcome>,
    /// Object balls each player has sunk, in the order they dropped.
    pub potted: [Vec<u8>; 2],
}

impl Default for Rules {
    fn default() -> Self {
        Self::new()
    }
}

impl Rules {
    pub fn new() -> Self {
        Self {
            current_player: 0,
            groups: [None, None],
            winner: None,
            ball_in_hand: false,
            last_outcome: None,
            potted: [Vec::new(), Vec::new()],
        }
    }

    /// Object balls this player has potted, ready for the HUD tray.
    pub fn potted_by(&self, player: usize) -> &[u8] {
        &self.potted[player]
    }

    pub fn opponent(&self) -> usize {
        1 - self.current_player
    }

    pub fn open_table(&self) -> bool {
        self.groups[0].is_none() && self.groups[1].is_none()
    }

    pub fn group_of_player(&self, player: usize) -> Option<Group> {
        self.groups[player]
    }

    fn owns(&self, player: usize, ball: u8) -> bool {
        match self.groups[player] {
            Some(group) => group_of(ball) == Some(group),
            None => group_of(ball).is_some(),
        }
    }

    fn assign_groups(&mut self, shooter: usize, potted: &[u8]) {
        let solids = potted
            .iter()
            .filter(|id| group_of(**id) == Some(Group::Solids))
            .count();
        let stripes = potted
            .iter()
            .filter(|id| group_of(**id) == Some(Group::Stripes))
            .count();
        // A shot that pots both types leaves the table open.
        let group = match (solids > 0, stripes > 0) {
            (true, false) => Group::Solids,
            (false, true) => Group::Stripes,
            _ => return,
        };
        self.groups[shooter] = Some(group);
        self.groups[1 - shooter] = Some(match group {
            Group::Solids => Group::Stripes,
            Group::Stripes => Group::Solids,
        });
    }

    /// Applies the standard eight ball foul rules for the shooter's turn.
    fn detect_foul(&self, shooter: usize, report: &ShotReport) -> Option<Foul> {
        if report.cue_potted {
            return Some(Foul::CueBallPotted);
        }
        let Some(first) = report.first_contact else {
            return Some(Foul::NoBallContact);
        };
        // A pot excuses the cushion requirement; anything else must send a ball
        // to a rail or the table is being pushed around.
        if !report.rail_after_contact && report.potted.is_empty() {
            return Some(Foul::NoRailContact);
        }
        let legal_first = match self.groups[shooter] {
            // Open table: anything but the eight may be struck first.
            None => first != 8,
            // On the black: only the eight is a legal first contact.
            Some(_) if report.shooter_cleared => first == 8,
            Some(group) => group_of(first) == Some(group),
        };
        if legal_first {
            None
        } else {
            Some(Foul::WrongBallFirst)
        }
    }

    pub fn resolve_shot(&mut self, report: &ShotReport) -> ShotOutcome {
        if let Some(winner) = self.winner {
            return ShotOutcome::Win { player: winner };
        }

        let shooter = self.current_player;
        let eight_potted = report.potted.contains(&8);
        let foul = self.detect_foul(shooter, report);

        // The eight ball coming down on the break is re-spotted, never a result.
        if eight_potted && !report.on_break {
            let outcome = if foul.is_none() && report.shooter_cleared {
                self.winner = Some(shooter);
                ShotOutcome::Win { player: shooter }
            } else {
                let opponent = 1 - shooter;
                self.winner = Some(opponent);
                ShotOutcome::Lose { player: shooter }
            };
            self.last_outcome = Some(outcome);
            return outcome;
        }

        if self.open_table() && !report.potted.is_empty() {
            self.assign_groups(shooter, &report.potted);
        }

        for ball in &report.potted {
            if *ball != 0 && *ball != 8 && !self.potted[shooter].contains(ball) {
                self.potted[shooter].push(*ball);
            }
        }
        self.potted[shooter].sort_unstable_by_key(|ball| (is_striped_ball(*ball), *ball));

        let own_potted = report
            .potted
            .iter()
            .any(|id| *id != 8 && self.owns(shooter, *id));

        let outcome = match foul {
            Some(reason) => {
                self.current_player = 1 - shooter;
                self.ball_in_hand = true;
                ShotOutcome::Foul(reason)
            }
            None if own_potted => ShotOutcome::Continue,
            None => {
                self.current_player = 1 - shooter;
                ShotOutcome::TurnOver
            }
        };

        self.last_outcome = Some(outcome);
        outcome
    }
}

/// True for balls `9..=15`, used only for tidy ordering in the tray.
fn is_striped_ball(id: u8) -> bool {
    (9..=15).contains(&id)
}

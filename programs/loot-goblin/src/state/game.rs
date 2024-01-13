use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::utils::Dice;

#[account(zero_copy)]
#[derive(Debug, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub struct Game {
    pub bump: u8,
    pub creator: Pubkey,
    pub id: u8,
    // 0 - new game
    // 1 - setup goblins
    // 2 - greed roll
    // 3 - crawl started
    // 4 - crawl ended
    pub game_phase: u8,
    pub game_rounds: u8,
    pub num_goblins: u8,
    pub turn_count: u8,
    pub turn_goblin: u8,
    pub turn_phase: u8,  // 0 - rummage, 1 - bribe, 2 - move
    pub turn_events: u8, // number of events this turn
    pub rummage_success_min: u8,
    pub event: u8,                   // 0 - none, 1+ - things that happen
    pub event_side_effects: [u8; 2], // events give the turn goblin 2 choices with possible side-effects
    pub event_outcome: u8,           // 0 - none, 1+ EventOutcome
    pub aftermath_option: u8,        // 0 - choose, 1 - must continue, 2 - must stop
    pub hero_bribe_rates: [u8; 4],   // thief, wizard, warrior (defends based on roll), merchant
    pub available_items: [u8; 4], // ring of reflect, healing potion, shield, cursed scroll (2x damage)
    pub goblins: [Goblin; 4],
}
impl Game {
    pub const SIZE: usize = 8 + // discriminator
        32 + // creator
        1 + // bump
        1 + // id
        1 + // game_phase
        1 + // game_rounds
        1 + // num_goblins
        1 + // turn_count
        1 + // turn_goblin
        1 + // turn_phase
        1 + // turn_events
        1 + // rummage_success_min
        1 + // event
        4 + // event_side_effects (len)
        2 + // event_side_effects (entries)
        1 + // event_outcome
        1 + // aftermath_option
        4 + // hero_bribe_rates (len)
        4 + // hero_bribe_rates (entries)
        4 + // available_items (len)
        4 + // available_items (entries)
        4 + // goblins (len)
        4 * ( // goblins (entries)
            32 + // player
            1 + // health
            1 + // luck
            1 + // greed
            1 + // last_roll
            1 + // last_roll_at
            1 + // held_item
            4 + // loot_bag (len)
            32 // loot_bag (entries)
        );
    pub const MIN_PLAYERS: usize = 1;
    pub const MAX_PLAYERS: usize = 4;
    pub const GAME_PHASE_NEW_GAME: u8 = 0;
    pub const GAME_PHASE_RECRUIT_GOBLINS: u8 = 1;
    pub const GAME_PHASE_FIND_GREEDIEST: u8 = 2;
    pub const GAME_PHASE_CRAWL_STARTED: u8 = 3;
    pub const GAME_PHASE_CRAWL_ENDED: u8 = 4;
    pub const TURN_PHASE_RUMMAGE: u8 = 0;
    pub const TURN_PHASE_BRIBE: u8 = 1;
    pub const TURN_PHASE_ITEM: u8 = 2; // choose to use item or not
    pub const TURN_PHASE_EVENT: u8 = 3; // generate next event + choices
    pub const TURN_PHASE_OUTCOME: u8 = 4; // handle outcome
    pub const TURN_PHASE_AFTERMATH: u8 = 5; // handle choice continue or stop
    pub const TURN_PHASE_SLAP_FIGHT: u8 = 6; // optional
    pub const AFTERMATH_OPTION_EITHER: u8 = 0;
    pub const AFTERMATH_OPTION_CONTINUE: u8 = 1;
    pub const AFTERMATH_OPTION_STOP: u8 = 2;
    pub const AFTERMATH_OPTION_LEN: u8 = 3;
    pub fn ptr(&self) -> *const Game {
        self as *const Game
    }
    pub fn mut_ptr(&mut self) -> *mut Game {
        self as *mut Game
    }
    pub fn set_event_outcome(&mut self, event_outcome: EventOutcome) {
        self.event_outcome = event_outcome as u8;
    }
    pub fn get_turn_goblin(&self) -> &mut Goblin {
        let goblins_ptr = self.goblins.as_ptr() as *mut Goblin;
        let i = (self.turn_goblin % self.num_goblins) as usize;
        unsafe { &mut *goblins_ptr.add(i) }
    }
    pub fn get_goblin_mut(&self, i: usize) -> &mut Goblin {
        let goblins_ptr = self.goblins.as_ptr() as *mut Goblin;
        unsafe { &mut *goblins_ptr.add(i) }
    }
    pub fn new_random_event(&mut self, dice: &mut Dice) {
        self.event = 0;
        while self.event == 0 {
            self.event = dice.roll(Dice::MAX);
        }
        self.turn_events += 1;
        // randomize event choice side-effects
        let option_a = dice.roll(Dice::D10);
        let option_b = dice.roll(Dice::D10);
        self.event_side_effects = [option_a, option_b];
    }
    pub fn advance_to_next_goblin(&mut self) {
        self.turn_goblin = (self.turn_goblin + 1) % self.num_goblins;
    }
    pub fn start_turn(&mut self) {
        self.turn_phase = Game::TURN_PHASE_RUMMAGE;
        self.turn_events = 0;
        self.turn_count += 1;
        let mut dice = Dice::new();
        self.rummage_success_min = dice.roll(Dice::D10);
        let goblin = self.get_turn_goblin();
        if goblin.health == 0 {
            goblin.health = Goblin::MAX_HEALTH;
        }
    }
}

#[derive(Debug, Copy, Clone, Zeroable, Pod, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
#[repr(C)]
pub struct Goblin {
    pub player: Pubkey,
    pub health: u8,
    pub luck: u8,
    pub greed: u8,
    pub last_roll: u8,
    pub last_roll_at: u8,
    pub held_item: u8, // held item id
    pub loot_bag: [u8; 32],
}
impl Goblin {
    pub const MAX_HEALTH: u8 = 2;
    pub fn init(&mut self, player: Pubkey) {
        self.player = player;
        self.health = Self::MAX_HEALTH;
    }
    pub fn can_be_controlled_by(&self, player: Pubkey) -> bool {
        // Anyone can control a CPU goblin
        if self.player == Pubkey::default() {
            return true;
        }
        // Check if signer is goblin's player
        return self.player == player;
    }
    pub fn add_loot(&mut self, loot: u8) -> bool {
        for n in &mut self.loot_bag {
            if *n == 0 {
                *n = loot.min(Dice::LOOT + 1);
                return true;
            }
        }
        return false;
    }
    pub fn add_random_loot(&mut self, dice: &mut Dice) -> bool {
        for n in &mut self.loot_bag {
            if *n == 0 {
                *n = 1 + dice.roll(Dice::LOOT);
                return true;
            }
        }
        return false;
    }
    pub fn take_least_valuable_loot(&mut self) -> u8 {
        let loot = *self.loot_bag.iter().min().unwrap();
        self.loot_bag
            .iter_mut()
            .find(|n| **n == loot)
            .map(|n| *n = 0);
        return loot;
    }
    pub fn add_random_item(&mut self, dice: &mut Dice) {
        self.held_item = 1 + dice.roll(Dice::ITEM);
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum EventOutcome {
    GetLoot = 0,
    GetItem,
    StealLoot,
    StealItem,
    Heal,
    BoostLuck,
    ReduceGreed,
    LoseLoot,
    LoseItem,
    LootGotStolen,
    ItemGotStolen,
    SlapFight,
    GetAttacked,
    OK,
}
impl EventOutcome {
    pub const LEN: usize = EventOutcome::OK as usize + 1;
}

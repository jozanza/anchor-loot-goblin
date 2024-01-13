use std::collections::HashSet;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction, sysvar::rent::Rent};

// https://solanacookbook.com/references/programs.html#how-to-change-account-size
pub fn resize_account<'a>(
    target_account: &AccountInfo<'a>,
    funding_account: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    new_size: usize,
) -> Result<()> {
    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(new_size);

    let lamports_diff = new_minimum_balance.saturating_sub(target_account.lamports());
    invoke(
        &system_instruction::transfer(funding_account.key, target_account.key, lamports_diff),
        &[
            funding_account.clone(),
            target_account.clone(),
            system_program.clone(),
        ],
    )?;

    target_account.realloc(new_size, false)?;

    Ok(())
}

pub fn xorshift64(seed: u64) -> u64 {
    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

pub struct Dice(u64);
impl Dice {
    pub const D10: u8 = 10;
    pub const COIN_FLIP: u8 = 1;
    pub const LOOT: u8 = 5;
    pub const ITEM: u8 = 8;
    pub const ONE_HUNDO: u8 = 100;
    pub const MAX: u8 = 255;
    pub fn new() -> Self {
        let clock = Clock::get().expect("couldn't get clock");
        Self(xorshift64(clock.slot))
    }
    pub fn roll(&mut self, sides: u8) -> u8 {
        let sides = sides as u64;
        let result = self.0 % sides;
        let seed = self.0.saturating_add(result).saturating_add(sides);
        self.0 = xorshift64(seed);
        result as u8
    }
    pub fn roll_unique(&mut self, sides: u8, num_rolls: usize) -> HashSet<u8> {
        let mut rolls = HashSet::new();
        while rolls.len() < num_rolls {
            let n = self.roll(sides);
            rolls.insert(n as u8);
        }
        rolls
    }
}

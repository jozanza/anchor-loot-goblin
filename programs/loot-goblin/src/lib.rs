use anchor_lang::prelude::*;
pub use state::*;

mod error;
mod seeds;
mod state;
mod utils;

declare_id!("9yCzP1smsGZmbNyTn87bgLM8wGdj5GcomXZHfQ8JXggZ");

#[program]
pub mod loot_goblin {
    use super::*;
    use error::LootGoblinError;
    use state::{EventOutcome, Goblin};
    use utils::Dice;

    /// Initialize a new [Game].
    pub fn create_game(ctx: Context<CreateGame>, game_id: u8, game_rounds: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_init()?;
        // Assign creator, bump, and id
        game.creator = ctx.accounts.creator.key();
        game.bump = *ctx.bumps.get("game").unwrap();
        game.id = game_id;
        // Assign some initial values to game state
        game.game_rounds = game_rounds;
        game.game_phase = Game::GAME_PHASE_RECRUIT_GOBLINS;
        Ok(())
    }

    pub fn recruit_goblins(
        ctx: Context<RecruitGoblins>,
        num_goblins: u8,
        players: Vec<Pubkey>,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Check if signer is game creator
        if ctx.accounts.creator.key() != game.creator {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_RECRUIT_GOBLINS {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check num players
        let num_players = players.len();
        if num_players < Game::MIN_PLAYERS {
            return err!(LootGoblinError::TooFewPlayers);
        }
        if num_players > Game::MAX_PLAYERS {
            return err!(LootGoblinError::TooManyPlayers);
        }
        if num_players > num_goblins as usize {
            return err!(LootGoblinError::TooManyPlayers);
        }
        game.num_goblins = num_goblins;
        // Init goblins
        let mut i = 0;
        for player in players {
            game.goblins[i].init(player);
            i += 1;
        }
        // Move to next phrase
        game.game_phase = Game::GAME_PHASE_FIND_GREEDIEST;
        Ok(())
    }

    pub fn find_greediest_goblin(ctx: Context<FindGreediestGoblin>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_FIND_GREEDIEST {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Get a unique roll per goblin
        let mut dice = Dice::new();
        let rolls = dice.roll_unique(Dice::D10, game.num_goblins as usize);
        // Update goblin greed
        let mut max_greed = 0;
        let mut max_index = 0;
        for (i, greed) in rolls.iter().enumerate() {
            game.goblins[i].greed = *greed;
            game.goblins[i].last_roll = *greed;
            game.goblins[i].last_roll_at = game.turn_count;
            if *greed > max_greed {
                max_greed = *greed;
                max_index = i;
            }
        }
        // Let the crawl commence!
        game.game_phase = Game::GAME_PHASE_CRAWL_STARTED;
        // The greediest goblin goes first
        game.turn_goblin = max_index as u8;
        game.start_turn();
        Ok(())
    }

    pub fn rummage_through_loot_sack(ctx: Context<RummageThroughLootSack>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = game.get_turn_goblin();
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_RUMMAGE {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        // Do rummage roll
        let mut dice = Dice::new();
        if dice.roll(Dice::D10) >= game.rummage_success_min {
            goblin.add_random_loot(&mut dice);
        }
        // Skipping bribe phase for now
        if true {
            // Move to outcome phase w new event
            game.new_random_event(&mut dice);
            game.turn_phase = Game::TURN_PHASE_OUTCOME;
            return Ok(());
        }
        // Move to bribe phase
        game.turn_phase = Game::TURN_PHASE_BRIBE;
        Ok(())
    }

    pub fn bribe_hero(
        ctx: Context<BribeHero>,
        did_bribe: bool,
        hero_index: u32,
        loot_index: u32,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = game.get_turn_goblin();
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_BRIBE {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        // Check if the player is bribing a hero
        if did_bribe {
            // TODO: handle bribe logic
            // ...
        }
        if goblin.held_item == 0 {
            // Move to outcome phase w new event
            let mut dice = Dice::new();
            game.new_random_event(&mut dice);
            game.turn_phase = Game::TURN_PHASE_OUTCOME;
            return Ok(());
        }
        // Move to item phase
        game.turn_phase = Game::TURN_PHASE_ITEM;
        Ok(())
    }

    pub fn use_item(ctx: Context<UseItem>, use_item: bool) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = game.get_turn_goblin();
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_ITEM {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        if use_item {
            // TODO: handle item usage logic
            // ...
        }
        // Move to event phase
        // game.turn_phase = Game::TURN_PHASE_EVENT;
        // Move to outcome phase w new event
        let mut dice = Dice::new();
        game.new_random_event(&mut dice);
        game.turn_phase = Game::TURN_PHASE_OUTCOME;
        Ok(())
    }

    pub fn trigger_event(ctx: Context<TriggerEvent>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = unsafe { (*game.ptr()).get_turn_goblin() };
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_EVENT {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        // Generate a new event
        let mut dice = Dice::new();
        game.new_random_event(&mut dice);
        Ok(())
    }

    pub fn determine_outcome(ctx: Context<DetermineOutcome>, choice: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = unsafe { (*game.ptr()).get_turn_goblin() };
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_OUTCOME {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        // Handle choice side-effects
        let i = choice as usize % game.event_side_effects.len();
        match game.event_side_effects[i] {
            _side_effect_id => {
                // TODO
                // ...
            }
        }
        // Make sure goblin didn't faint from side-effects
        if goblin.health == 0 {
            // Start the next goblin's turn
            game.advance_to_next_goblin();
            game.start_turn();
            return Ok(());
        }
        // Calculate rich tax (richer goblins are less lucky)
        let total_loot: u8 = goblin.loot_bag.iter().sum();
        let rich_tax = total_loot / 10;
        // Set outcome probabilities
        let mut weights = [0u8; EventOutcome::LEN];
        // Good stuff
        weights[EventOutcome::GetLoot as usize] = 10 + goblin.luck;
        weights[EventOutcome::GetItem as usize] = 1 + goblin.luck;
        weights[EventOutcome::StealLoot as usize] = 1 + goblin.luck;
        weights[EventOutcome::StealItem as usize] = 1 + goblin.luck;
        weights[EventOutcome::Heal as usize] = 1 + goblin.luck;
        weights[EventOutcome::BoostLuck as usize] = 1 + goblin.greed;
        weights[EventOutcome::ReduceGreed as usize] = 1 + goblin.greed;
        // Bad + neutral stuff
        weights[EventOutcome::LoseLoot as usize] = 10 + goblin.greed + rich_tax;
        weights[EventOutcome::LoseItem as usize] = 1 + goblin.greed + rich_tax;
        weights[EventOutcome::LootGotStolen as usize] = 1 + goblin.greed + rich_tax;
        weights[EventOutcome::ItemGotStolen as usize] = 1 + goblin.greed + rich_tax;
        weights[EventOutcome::SlapFight as usize] = 1 + goblin.greed + rich_tax;
        weights[EventOutcome::GetAttacked as usize] = 1 + (goblin.greed * game.turn_events);
        weights[EventOutcome::OK as usize] = 1 + goblin.luck;
        // If the goblin isn't risking, reduce reward and make OK outcome very likely
        // if !risk_it {
        //     weights[EventOutcome::GetLoot as usize] = 0;
        //     weights[EventOutcome::GetItem as usize] = 0;
        //     weights[EventOutcome::StealLoot as usize] = 0;
        //     weights[EventOutcome::StealItem as usize] = 0;
        //     weights[EventOutcome::Heal as usize] = 0;
        //     weights[EventOutcome::BoostLuck as usize] = 0;
        //     weights[EventOutcome::ReduceGreed as usize] = 0;
        //     weights[EventOutcome::OK as usize] = 40;
        // }
        // Calculate outcome
        let total_weight: u8 = weights.iter().sum();
        let mut dice = Dice::new();
        let roll = dice.roll(total_weight);
        let mut outcome = EventOutcome::OK;
        let mut offset = 0;
        for (i, weight) in weights.iter().enumerate() {
            if roll >= offset && roll < offset + weight {
                outcome = unsafe { std::mem::transmute(i as u8) };
                break;
            }
            offset += weight;
        }
        game.set_event_outcome(outcome);
        msg!("{:?}", outcome);
        // Handle outcome
        match outcome {
            EventOutcome::GetLoot => {
                goblin.add_random_loot(&mut dice);
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::GetItem => {
                goblin.add_random_item(&mut dice);
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::StealLoot => {
                let i = dice.roll(game.num_goblins as u8) as usize;
                let victim = game.get_goblin_mut(i);
                let loot = victim.take_least_valuable_loot();
                victim.luck = victim.luck.saturating_add(1);
                goblin.add_loot(loot);
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::StealItem => {
                let i = dice.roll(game.num_goblins as u8) as usize;
                let victim = game.get_goblin_mut(i);
                let item = victim.held_item;
                victim.held_item = 0;
                if item > 0 {
                    goblin.held_item = item;
                    goblin.greed = goblin.greed.saturating_add(1);
                }
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::Heal => {
                goblin.health = goblin.health.saturating_add(1).min(Goblin::MAX_HEALTH);
                game.aftermath_option = Game::AFTERMATH_OPTION_STOP;
            }
            EventOutcome::BoostLuck => {
                goblin.luck = goblin.luck.saturating_add(1);
                game.aftermath_option = Game::AFTERMATH_OPTION_STOP;
            }
            EventOutcome::ReduceGreed => {
                goblin.greed = goblin.greed.saturating_sub(1);
                game.aftermath_option = Game::AFTERMATH_OPTION_STOP;
            }
            EventOutcome::LoseLoot => {
                let _loot = goblin.take_least_valuable_loot();
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::LoseItem => {
                goblin.held_item = 0;
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::LootGotStolen => {
                let loot = goblin.take_least_valuable_loot();
                let i = dice.roll(game.num_goblins as u8) as usize;
                game.get_goblin_mut(i).add_loot(loot);
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::ItemGotStolen => {
                let thief_index = dice.roll(game.num_goblins as u8) as usize;
                game.get_goblin_mut(thief_index).held_item = goblin.held_item;
                goblin.held_item = 0;
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
            EventOutcome::SlapFight => {
                // Immediately move to slap fight phase
                game.turn_phase = Game::TURN_PHASE_SLAP_FIGHT;
                return Ok(());
            }
            EventOutcome::GetAttacked => {
                goblin.health = goblin.health.saturating_sub(1);
                let _loot = goblin.take_least_valuable_loot();
                game.aftermath_option = Game::AFTERMATH_OPTION_STOP;
            }
            EventOutcome::OK => {
                // Nothing happens! :)
                game.aftermath_option = dice.roll(Game::AFTERMATH_OPTION_LEN);
            }
        }
        // Move to the aftermath phase
        game.turn_phase = Game::TURN_PHASE_AFTERMATH;
        Ok(())
    }

    pub fn make_aftermath_decision(ctx: Context<MakeAftermathDecision>, choice: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = unsafe { (*game.ptr()).get_turn_goblin() };
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_AFTERMATH {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        // Continue or stop
        match (game.aftermath_option, choice) {
            (Game::AFTERMATH_OPTION_CONTINUE, _)
            | (Game::AFTERMATH_OPTION_EITHER, Game::AFTERMATH_OPTION_CONTINUE) => {
                if goblin.held_item == 0 {
                    let mut dice = Dice::new();
                    // Move to outcome phase w new event
                    game.new_random_event(&mut dice);
                    game.turn_phase = Game::TURN_PHASE_OUTCOME;
                    return Ok(());
                }
                game.turn_phase = Game::TURN_PHASE_ITEM;
                return Ok(());
            }
            _ => {
                game.advance_to_next_goblin();
                game.start_turn();
                return Ok(());
            }
        }
    }

    pub fn slap_fight(ctx: Context<SlapFight>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let mut game = game.load_mut()?;
        // Ensure goblin can be controlled by signer
        let goblin = unsafe { (*game.ptr()).get_turn_goblin() };
        if !goblin.can_be_controlled_by(ctx.accounts.signer.key()) {
            return err!(LootGoblinError::InvalidAuthority);
        }
        // Check game phase
        if game.game_phase != Game::GAME_PHASE_CRAWL_STARTED {
            return err!(LootGoblinError::WrongGamePhase);
        }
        // Check turn phase
        if game.turn_phase != Game::TURN_PHASE_SLAP_FIGHT {
            return err!(LootGoblinError::WrongTurnPhase);
        }
        // Each goblin rolls, find the highest and lowest rolls
        let mut dice = Dice::new();
        let rolls = dice.roll_unique(Dice::D10, game.num_goblins as usize);
        let mut highest_roll = 0;
        let mut lowest_roll = u8::MAX;
        let mut highest_goblin_index = 0;
        let mut lowest_goblin_index = 0;
        for (i, roll) in rolls.iter().enumerate() {
            game.goblins[i].last_roll = *roll;
            game.goblins[i].last_roll_at = game.turn_count;
            if *roll > highest_roll {
                highest_roll = *roll;
                highest_goblin_index = i;
            }
            if *roll < lowest_roll {
                lowest_roll = *roll;
                lowest_goblin_index = i;
            }
        }
        // Increase all goblin greed
        for goblin in &mut game.goblins {
            goblin.greed = goblin.greed.saturating_add(1);
        }
        // The goblin with the highest roll takes loot from the one with the lowest roll
        if highest_goblin_index != lowest_goblin_index {
            let loot = game.goblins[lowest_goblin_index].take_least_valuable_loot();
            game.goblins[highest_goblin_index].add_loot(loot);
            // decrease loser greed
            game.goblins[lowest_goblin_index].greed =
                game.goblins[lowest_goblin_index].greed.saturating_sub(1);
            // Increase winner greed
            game.goblins[highest_goblin_index].greed =
                game.goblins[highest_goblin_index].greed.saturating_add(1);
        }

        // Start the next goblin's turn
        game.advance_to_next_goblin();
        game.start_turn();
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(game_id: u8, game_rounds: u8)]
pub struct CreateGame<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        space = Game::SIZE,
        seeds = [seeds::GAME, creator.key().as_ref(), &[game_id]],
        bump,
    )]
    pub game: AccountLoader<'info, Game>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(num_goblins: u8, players: Vec<Pubkey>)]
pub struct RecruitGoblins<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            creator.key().as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
pub struct FindGreediestGoblin<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            creator.key().as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
pub struct RummageThroughLootSack<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
#[instruction(did_bribe: bool, hero_index: u32, loot_index: u32)]
pub struct BribeHero<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
#[instruction(use_item: bool)]
pub struct UseItem<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
pub struct TriggerEvent<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
pub struct DetermineOutcome<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
#[instruction(choice: u8)]
pub struct MakeAftermathDecision<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

#[derive(Accounts)]
pub struct SlapFight<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            seeds::GAME,
            game.load()?.creator.as_ref(),
            &[game.load()?.id],
        ],
        bump = game.load()?.bump,
    )]
    pub game: AccountLoader<'info, Game>,
}

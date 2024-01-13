import * as anchor from "@coral-xyz/anchor";
import { Program, SystemProgram } from "@coral-xyz/anchor";
import { LootGoblin } from "../target/types/loot_goblin";
import { expect } from "chai";

type Game = Awaited<
  ReturnType<Program<LootGoblin>["account"]["game"]["fetch"]>
>;

const GAME_PHASE_NEW_GAME = 0;
const GAME_PHASE_RECRUIT_GOBLINS = 1;
const GAME_PHASE_FIND_GREEDIEST = 2;
const GAME_PHASE_CRAWL_STARTED = 3;
const GAME_PHASE_CRAWL_ENDED = 4;
const TURN_PHASE_RUMMAGE = 0;
const TURN_PHASE_BRIBE = 1;
const TURN_PHASE_ITEM = 2;
const TURN_PHASE_EVENT = 3;
const TURN_PHASE_OUTCOME = 4;
const TURN_PHASE_AFTERMATH = 5;
const TURN_PHASE_SLAP_FIGHT = 6;
const AFTERMATH_OPTION_EITHER = 0;
const AFTERMATH_OPTION_CONTINUE = 1;
const AFTERMATH_OPTION_STOP = 2;

enum EventOutcome {
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

describe("loot-goblin", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.LootGoblin as Program<LootGoblin>;

  // Game id
  const gameId = 0;

  // Game pubkey
  const [gamePubkey] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("game"),
      provider.wallet.publicKey.toBuffer(),
      Buffer.from([gameId]),
    ],
    program.programId
  );

  // const [gamePubkey2] = anchor.web3.PublicKey.findProgramAddressSync(
  //   [
  //     Buffer.from("game"),
  //     provider.wallet.publicKey.toBuffer(),
  //     Buffer.from([1]),
  //   ],
  //   program.programId
  // );

  let prevGame: Game | null = null;

  it("Creates a new game", async () => {
    // Define game parameters
    const gameRounds = 10;

    // Create the new game
    // const x = program.methods.createGame(gameId, gameRounds).accounts({
    //   game: gamePubkey,
    //   creator: provider.wallet.publicKey,
    // });
    // const ins = await x.instruction();
    // console.log(ins);
    // console.log([...ins.data]);
    await program.methods
      .createGame(gameId, gameRounds)
      .accounts({
        game: gamePubkey,
        creator: provider.wallet.publicKey,
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the newly created game account
    const game = await program.account.game.fetch(gamePubkey);

    // Check if the game has been initialized correctly
    expect(game.id).to.equal(gameId);
    expect(game.gameRounds).to.equal(gameRounds);
    expect(game.turnCount).to.equal(0);
    expect(game.gamePhase).to.equal(GAME_PHASE_RECRUIT_GOBLINS);
    prevGame = game;
  });

  it("Recruits goblins", async () => {
    // Define the number of goblins and their public keys
    const numGoblins = 4;
    const players = [provider.wallet.publicKey];

    // Recruit goblins
    await program.methods
      .recruitGoblins(numGoblins, players)
      .accounts({
        game: gamePubkey,
        creator: provider.wallet.publicKey,
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Check if the goblins have been added correctly
    expect(game.numGoblins).to.equal(numGoblins);
    for (let i = 0; i < numGoblins; i++) {
      const actual = game.goblins[i].player.toBase58();
      const expected = players[i]
        ? players[i].toBase58()
        : anchor.web3.PublicKey.default.toBase58();
      expect(actual).to.equal(expected);
    }
    expect(game.gamePhase).to.equal(GAME_PHASE_FIND_GREEDIEST);
    prevGame = game;
  });

  it("Finds the greediest goblin", async () => {
    // Call the findGreediestGoblin method
    await program.methods
      .findGreediestGoblin()
      .accounts({
        game: gamePubkey,
        creator: provider.wallet.publicKey,
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Perform checks to validate that the greediest goblin is determined correctly
    const greed = game.goblins.map((g) => g.greed);
    const greediestIndex = greed.indexOf(Math.max(...greed));
    expect(game.turnGoblin).to.equal(greediestIndex);
    expect(game.gamePhase).to.equal(GAME_PHASE_CRAWL_STARTED);
    expect(game.turnPhase).to.equal(TURN_PHASE_RUMMAGE);
    prevGame = game;
  });

  it("Rummages through loot sack", async () => {
    // Call the rummageThroughLootSack method for the current turn's goblin
    await program.methods
      .rummageThroughLootSack()
      .accounts({
        game: gamePubkey,
        signer: provider.wallet.publicKey, // Assuming the wallet is controlling the current turn's goblin
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Check if the goblin's loot bag has been updated
    // This check will depend on how your rummage logic works
    // For example, check if the goblin's loot bag contains new items
    const goblin = game.goblins[game.turnGoblin];
    if (game.rummageSuccessMin > goblin.lastRoll) {
      expect(goblin.lootBag[0]).to.equal(goblin.lastRoll);
    }

    // Check if the turn phase has been updated to the bribe phase
    expect(game.turnPhase).to.equal(TURN_PHASE_BRIBE);
    prevGame = game;
  });

  it("Bribes a hero", async () => {
    // Define the hero index to bribe and the loot index to use for the bribe
    const heroIndex = 0; // Example hero index
    const lootIndex = 0; // Example loot index (assuming the goblin has loot to bribe with)

    // Call the bribeHero method
    await program.methods
      .bribeHero(true, heroIndex, lootIndex)
      .accounts({
        game: gamePubkey,
        signer: provider.wallet.publicKey, // Assuming the wallet is controlling the current turn's goblin
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Perform checks to validate the bribe
    expect(game.turnPhase).to.equal(TURN_PHASE_ITEM);
    prevGame = game;
  });

  it("Uses an item", async () => {
    // Simulate using an item
    const useItem = false;

    // Call the useItem method
    await program.methods
      .useItem(useItem)
      .accounts({
        game: gamePubkey,
        signer: provider.wallet.publicKey, // Assuming the wallet is controlling the current turn's goblin
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Perform checks to validate the item usage
    expect(game.turnPhase).to.equal(TURN_PHASE_EVENT);
    prevGame = game;
  });

  it("Triggers an event", async () => {
    // Call the triggerEvent method
    await program.methods
      .triggerEvent()
      .accounts({
        game: gamePubkey,
        signer: provider.wallet.publicKey, // Assuming the wallet is controlling the current turn's goblin
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Perform checks to validate the event trigger
    expect(game.event).to.be.greaterThan(0);
    expect(game.turnEvents).to.equal(1);
    expect(game.turnPhase).to.equal(TURN_PHASE_OUTCOME);
    prevGame = game;
  });

  it("Determines the outcome of an event", async () => {
    // Simulate making a choice for the event outcome
    const choice = 0; // continue

    // Call the determineOutcome method
    await program.methods
      .determineOutcome(choice)
      .accounts({
        game: gamePubkey,
        signer: provider.wallet.publicKey, // Assuming the wallet is controlling the current turn's goblin
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);
    // Perform checks to validate the outcome
    switch (game.eventOutcome) {
      case EventOutcome.GetLoot: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        const prevLootTotal = prevGoblin.lootBag.reduce((acc, a) => acc + a, 0);
        const lootTotal = goblin.lootBag.reduce((acc, a) => acc + a, 0);
        expect(lootTotal).to.be.greaterThan(prevLootTotal);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.GetItem: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        expect(prevGoblin.heldItem).to.equal(0);
        expect(goblin.heldItem).to.be.greaterThan(0);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.StealLoot: {
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.heldItem).to.equal(0);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.Heal: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.health).to.equal(Math.min(2, prevGoblin.health + 1));
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.BoostLuck: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.luck).to.equal(prevGoblin.luck + 1);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.ReduceGreed: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.greed).to.equal(Math.max(0, prevGoblin.greed - 1));
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.LoseLoot: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        const prevLootTotal = prevGoblin.lootBag.reduce((acc, a) => acc + a, 0);
        const lootTotal = goblin.lootBag.reduce((acc, a) => acc + a, 0);
        expect(lootTotal).to.be.lessThan(prevLootTotal);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.LoseItem: {
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.heldItem).to.equal(0);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.LootGotStolen: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        const prevLootTotal = prevGoblin.lootBag.reduce((acc, a) => acc + a, 0);
        const lootTotal = goblin.lootBag.reduce((acc, a) => acc + a, 0);
        if (prevLootTotal === 0) {
          expect(lootTotal).to.equal(0);
        } else {
          expect(lootTotal).to.be.lessThan(prevLootTotal);
        }
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.ItemGotStolen: {
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.heldItem).to.equal(0);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.SlapFight: {
        expect(game.turnPhase).to.equal(TURN_PHASE_SLAP_FIGHT);
        break;
      }
      case EventOutcome.GetAttacked: {
        const prevGoblin = prevGame.goblins[prevGame.turnGoblin];
        const goblin = game.goblins[game.turnGoblin];
        expect(goblin.health).to.be.lessThan(prevGoblin.health);
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
      case EventOutcome.OK: {
        expect(game.turnPhase).to.equal(TURN_PHASE_AFTERMATH);
        break;
      }
    }
    prevGame = game;
  });

  it("Makes a decision in the aftermath of an event", async () => {
    // Simulate making a decision after the event
    const option =
      prevGame.aftermathOption === AFTERMATH_OPTION_STOP
        ? AFTERMATH_OPTION_STOP
        : AFTERMATH_OPTION_CONTINUE;

    // Call the makeAftermathDecision method
    await program.methods
      .makeAftermathDecision(option)
      .accounts({
        game: gamePubkey,
        signer: provider.wallet.publicKey, // Assuming the wallet is controlling the current turn's goblin
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    // Fetch the updated game account
    const game = await program.account.game.fetch(gamePubkey);

    // Perform checks to validate the aftermath decision
    if (option === AFTERMATH_OPTION_CONTINUE) {
      expect(game.turnGoblin).to.equal(prevGame.turnGoblin);
      expect(game.turnPhase).to.equal(TURN_PHASE_ITEM);
    } else {
      expect(game.turnGoblin).not.to.equal(prevGame.turnGoblin);
      expect(game.turnPhase).to.equal(TURN_PHASE_RUMMAGE);
    }
  });
});

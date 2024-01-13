use anchor_lang::prelude::*;

#[error_code]
pub enum LootGoblinError {
    /// Returned if the length of a parameter exceeds its allowed limits.
    #[msg("Exceeded max length for field.")]
    InvalidFieldLength,

    /// Returned if the wrong authority attempts to sign for an instruction
    #[msg("Invalid authority for instruction")]
    InvalidAuthority,

    /// Returned if an account that's expected to sign doesn't.
    #[msg("An expected signature isn't present")]
    MissingSignature,

    #[msg("An optional but expected account is missing")]
    MissingExpectedAccount,

    #[msg("Not in the correct game phase to perform this action")]
    WrongGamePhase,

    #[msg("Not in the correct turn phase to perform this action")]
    WrongTurnPhase,

    #[msg("Too many players.")]
    TooManyPlayers,

    #[msg("Too few players.")]
    TooFewPlayers,
}

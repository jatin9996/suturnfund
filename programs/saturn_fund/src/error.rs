use anchor_lang::prelude::*;

#[error]
pub enum SaturnFundError {
    #[msg("Insufficient funds in the holding account.")]
    InsufficientFunds,
    #[msg("Slippage tolerance exceeded.")]
    SlippageExceeded,
    #[msg("Unauthorized access.")]
    Unauthorized,
}

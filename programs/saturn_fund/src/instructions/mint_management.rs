// In mint_management.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

#[derive(Accounts)]
pub struct CreateMintAccount<'info> {
    #[account(init, payer = user, mint::decimals = 9, mint::authority = mint_authority)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_mint(ctx: Context<CreateMintAccount>) -> ProgramResult {
    msg!("Mint account created with mint authority set to program ID.");
    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, TokenAccount};

#[derive(Accounts)]
pub struct TransferWithFee<'info> {
    #[account(mut)]
    pub from: Account<'info, TokenAccount>, // SPL token account to debit
    #[account(mut)]
    pub to: Account<'info, TokenAccount>, // SPL token account to credit
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>, // SPL token account to receive the fee
    pub authority: Signer<'info>, // Authority must have permission to move tokens from 'from' account
    pub token_program: Program<'info, token::Token>, // SPL Token program
}

pub fn handler(ctx: Context<TransferWithFee>, amount: u64) -> ProgramResult {
    let fee = amount / 100; // 1% fee, demonstrating the fee deduction requirement
    let amount_after_fee = amount - fee;

    // Transfer the main amount to the 'to' account
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount_after_fee,
    )?;

    // Transfer the fee to the 'fee_account'
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.fee_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        fee,
    )?;

    Ok(())
}
// In fee_handling.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

pub fn transfer_with_fee(ctx: Context<TransferWithFee>, amount: u64, fee: u64) -> ProgramResult {
    let amount_after_fee = amount.checked_sub(fee).ok_or(ProgramError::InsufficientFunds)?;
    token::transfer(ctx.accounts.transfer_context(), amount_after_fee)?;
    token::transfer(ctx.accounts.fee_context(), fee)?;
    Ok(())
}

#[derive(Accounts)]
pub struct TransferWithFee<'info> {
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> TransferWithFee<'info> {
    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(self.token_program.to_account_info(), Transfer {
            from: self.from.to_account_info(),
            to: self.to.to_account_info(),
            authority: self.authority.to_account_info(),
        })
    }

    fn fee_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(self.token_program.to_account_info(), Transfer {
            from: self.from.to_account_info(),
            to: self.fee_account.to_account_info(),
            authority: self.authority.to_account_info(),
        })
    }
}

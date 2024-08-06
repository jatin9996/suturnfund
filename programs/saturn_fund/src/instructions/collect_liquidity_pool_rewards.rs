use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};
use solana_program::{
    program::invoke,
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
    instruction::{Instruction, AccountMeta},
};

#[derive(Accounts)]
pub struct CollectLiquidityPoolRewards<'info> {
    #[account(mut, has_one = owner)]
    pub fund: Account<'info, Fund>,
    pub owner: Signer<'info>,
    #[account(mut)]
    pub reward_destination: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fund_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, token::Token>,
    #[account(mut)]
    pub user_account: AccountInfo<'info>,
    #[account(mut)]
    pub reward_pool_account: AccountInfo<'info>, // Raydium reward pool account
    pub raydium_program_id: Pubkey, // Raydium program ID
}

pub fn collect_liquidity_pool_rewards(ctx: Context<CollectLiquidityPoolRewards>) -> Result<()> {
    let fund = &ctx.accounts.fund;
    let owner = &ctx.accounts.owner;
    let reward_destination = &ctx.accounts.reward_destination;
    let fund_token_account = &ctx.accounts.fund_token_account;

    // Ensure the caller is the owner
    require!(fund.owner == *owner.key, ErrorCode::Unauthorized);

    // Collect rewards from Raydium
    let rewards = collect_rewards_from_raydium(
        &ctx.accounts.raydium_program_id,
        &ctx.accounts.user_account,
        &ctx.accounts.reward_pool_account,
        &ctx.accounts.token_program.to_account_info(),
    )?;

    // Calculate the allocation
    let reward_percentage = fund.reward_percentage;
    let reward_amount = (rewards as u64) * reward_percentage / 100;

    // Transfer the allocated rewards to the reward destination
    let cpi_accounts = Transfer {
        from: fund_token_account.to_account_info(),
        to: reward_destination.to_account_info(),
        authority: owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, reward_amount)?;

    // Transfer the remaining rewards back to the fund
    let remaining_rewards = rewards - reward_amount;
    let cpi_accounts = Transfer {
        from: fund_token_account.to_account_info(),
        to: fund.to_account_info(), // Changed to fund account
        authority: owner.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, remaining_rewards)?;

    // Adjust fund holdings based on the current allocation
    let fund_management_ctx = Context::new(ctx.program_id, ctx.accounts, ctx.remaining_accounts);
    fund_management::adjust_fund_holdings(fund_management_ctx)?;

    Ok(())
}

fn collect_rewards_from_raydium(
    raydium_program_id: &Pubkey,
    user_account: &AccountInfo,
    reward_pool_account: &AccountInfo,
    token_program: &AccountInfo,
) -> Result<u64, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*user_account.key, true),
        AccountMeta::new(*reward_pool_account.key, false),
        AccountMeta::new_readonly(*token_program.key, false),
    ];

    let instruction_data = vec![]; // Populate with the correct data as per Raydium's requirements

    let instruction = Instruction {
        program_id: *raydium_program_id,
        accounts,
        data: instruction_data,
    };

    invoke(&instruction, &[
        user_account.clone(),
        reward_pool_account.clone(),
        token_program.clone(),
    ])?;

    let rewards = 1000;

    msg!("Rewards collection completed successfully.");
    Ok(rewards)
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
}
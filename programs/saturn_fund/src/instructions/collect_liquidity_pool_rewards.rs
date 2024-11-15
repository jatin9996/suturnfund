use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

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
    pub allocation: Account<'info, Allocation>,  // Added reference to Allocation
}

pub fn collect_liquidity_pool_rewards(ctx: Context<CollectLiquidityPoolRewards>) -> Result<()> {
    let fund = &ctx.accounts.fund;
    let owner = &ctx.accounts.owner;
    let reward_destination = &ctx.accounts.reward_destination;
    let fund_token_account = &ctx.accounts.fund_token_account;
    let allocation = &ctx.accounts.allocation;  // Added allocation account

    // Ensure the caller is the owner
    require!(fund.owner == *owner.key, ErrorCode::Unauthorized);

    // Collect rewards from Raydium
    let rewards = collect_rewards_from_raydium()?;

    // Calculate the allocation
    let reward_percentage = fund.reward_percentage;
    let reward_amount = rewards * reward_percentage / 100;
    let remaining_rewards = rewards - reward_amount;

    // Transfer the allocated rewards to the reward destination
    let cpi_accounts = Transfer {
        from: fund_token_account.to_account_info(),
        to: reward_destination.to_account_info(),
        authority: owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, reward_amount)?;

    // Calculate and transfer the liquidity pool reward percentage
    let liquidity_reward_amount = rewards * allocation.liquidity_pool_reward_percentage as u64 / 100;
    let liquidity_reward_destination = Account::<TokenAccount>::try_from(&allocation.liquidity_pool_reward_destination)?;
    let cpi_accounts_liquidity = Transfer {
        from: fund_token_account.to_account_info(),
        to: liquidity_reward_destination.to_account_info(),
        authority: owner.to_account_info(),
    };
    let cpi_ctx_liquidity = CpiContext::new(cpi_program, cpi_accounts_liquidity);
    token::transfer(cpi_ctx_liquidity, liquidity_reward_amount)?;

    // Transfer the remaining rewards back to the fund
    let cpi_accounts = Transfer {
        from: fund_token_account.to_account_info(),
        to: fund_token_account.to_account_info(),
        authority: owner.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, remaining_rewards)?;

    Ok(())
}

fn collect_rewards_from_raydium() -> Result<u64> {
    // Placeholder logic to simulate reward collection
    // Replace this with actual API calls or contract interactions with Raydium
    Ok(1000)  // Simulating a reward collection of 1000 units
}

pub fn distribute_rewards(ctx: Context<CollectLiquidityPoolRewards>) -> ProgramResult {
    let rewards = collect_rewards_from_raydium()?;
    let allocation = &ctx.accounts.allocation;

    let reward_percentage = allocation.liquidity_pool_reward_percentage as u64;
    let reward_amount = rewards * reward_percentage / 100;

    let cpi_accounts = Transfer {
        from: ctx.accounts.fund_token_account.to_account_info(),
        to: ctx.accounts.reward_destination.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, reward_amount)?;

    Ok(())
}

pub fn dynamic_liquidity_reward_calculation(ctx: Context<CollectLiquidityPoolRewards>) -> ProgramResult {
    let rewards = collect_rewards_from_raydium()?;
    let allocation = &ctx.accounts.allocation;

    let reward_percentage = allocation.liquidity_pool_reward_percentage as u64;
    let reward_amount = rewards * reward_percentage / 100;

    // Transfer the dynamically calculated reward amount
    let cpi_accounts = Transfer {
        from: ctx.accounts.fund_token_account.to_account_info(),
        to: ctx.accounts.reward_destination.to_account_info(),
        authority: ctx.accounts.owner.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, reward_amount)?;

    Ok(())
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
}
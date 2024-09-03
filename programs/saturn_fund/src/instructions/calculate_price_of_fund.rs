use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use crate::state::FundAccount;

#[derive(Accounts)]
pub struct CalculatePriceOfFund<'info> {
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub holding_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
}

pub fn handler(ctx: Context<CalculatePriceOfFund>) -> ProgramResult {
    let fund_account = &ctx.accounts.fund_account;
    let holding_account = &ctx.accounts.holding_account;
    let mint = &ctx.accounts.mint;

    // Step 1: Get how many liquidity pool tokens are held by the fund account
    let liquidity_pool_tokens = fund_account.amount;

    // Step 2: Calculate how many of the underlying tokens are equivalent to that number of liquidity pool tokens
    let underlying_tokens = calculate_underlying_tokens(liquidity_pool_tokens)?;

    // Step 3: Get the current market price of those tokens
    let market_price = get_current_market_price()?;

    // Step 4: Calculate the value of those tokens
    let value_of_tokens = underlying_tokens * market_price;

    // Step 5: Sum to get the total market value of the fund and also add the value of the holding account
    let total_market_value = value_of_tokens + holding_account.amount;

    // Get the number of tokens in circulation from the mint account
    let tokens_in_circulation = mint.supply;

    // Calculate the price of the fund
    let price_of_fund = total_market_value / tokens_in_circulation;

    msg!("Price of the fund: {}", price_of_fund);

    Ok(())
}

// Helper function to calculate the equivalent amount of underlying tokens
fn calculate_underlying_tokens(liquidity_pool_tokens: u64) -> Result<u64, ProgramError> {
    // logic: Assume each liquidity pool token is equivalent to 10 underlying tokens
    let underlying_tokens = liquidity_pool_tokens * 10;
    Ok(underlying_tokens) 
}

// Helper function to get the current market price of the tokens
fn get_current_market_price() -> Result<u64, ProgramError> {
    // Simulated market price for demonstration purposes
    let market_price: u64 = 100; // Assume the market price is 100 units

    Ok(market_price)
}
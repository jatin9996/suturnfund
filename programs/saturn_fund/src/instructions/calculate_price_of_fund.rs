use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use crate::state::FundAccount;
use crate::instructions::raydium_integration::get_current_market_price_from_raydium;

#[derive(Accounts)]
pub struct CalculatePriceOfFund<'info> {
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub holding_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    pub oracle_account: AccountInfo<'info>,
}

pub fn handler(ctx: Context<CalculatePriceOfFund>) -> ProgramResult {
    let fund_account = &ctx.accounts.fund_account;
    let holding_account = &ctx.accounts.holding_account;
    let mint = &ctx.accounts.mint;

    // Step 1: Get how many liquidity pool tokens are held by the fund account
    let liquidity_pool_tokens = fund_account.amount;

    // Step 2: Calculate how many of the underlying tokens are equivalent to that number of liquidity pool tokens
    let underlying_tokens = calculate_underlying_tokens(&ctx, liquidity_pool_tokens)?;

    // Step 3: Get the current market price of those tokens from Raydium's oracle
    let oracle_account_info = ctx.accounts.oracle_account.clone();
    let market_price = get_current_market_price_from_raydium(&oracle_account_info)?;

    // Step 4: Calculate the value of those tokens
    let value_of_tokens = underlying_tokens * market_price;

    // Step 5: Calculate the value of all different types of tokens held by the fund
    let total_value_of_all_tokens = calculate_total_value_of_all_tokens(&ctx.accounts)?;

    // Step 6: Sum to get the total market value of the fund and also add the value of the holding account
    let total_market_value = value_of_tokens + holding_account.amount + total_value_of_all_tokens;

    // Get the number of tokens in circulation from the mint account
    let tokens_in_circulation = mint.supply;

    // Calculate the price of the fund
    let price_of_fund = total_market_value / tokens_in_circulation;

    msg!("Price of the fund: {}", price_of_fund);

    Ok(())
}

// Helper function to calculate the equivalent amount of underlying tokens
fn calculate_underlying_tokens(ctx: &Context<CalculatePriceOfFund>, liquidity_pool_tokens: u64) -> Result<u64, ProgramError> {
    let conversion_ratio = ctx.accounts.fund_account.conversion_ratio; // Assuming conversion_ratio is a field in FundAccount
    let underlying_tokens = liquidity_pool_tokens.checked_mul(conversion_ratio).ok_or(ProgramError::Overflow)?;
    
    Ok(underlying_tokens)
}

// Helper function to calculate the total value of all tokens held by the fund
fn calculate_total_value_of_all_tokens(accounts: &Context<CalculatePriceOfFund>) -> Result<u64, ProgramError> {
    let mut total_value: u64 = 0;

    
    for token_account in accounts.iter() { // Adjust this iterator to match your context
        let market_price = get_current_market_price_from_raydium(&token_account.oracle_account)?; // Adjust the method to fetch the price
        let value = token_account.amount.checked_mul(market_price).ok_or(ProgramError::Overflow)?;
        total_value = total_value.checked_add(value).ok_or(ProgramError::Overflow)?;
    }

    Ok(total_value)
}
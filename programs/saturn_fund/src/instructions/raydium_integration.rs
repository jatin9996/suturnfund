use anchor_lang::prelude::*;
use solana_program::{
    program::invoke,
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use anchor_spl::token::{self, TokenAccount, Transfer};

// Function to fetch the current market price from Raydium's oracle
pub fn get_current_market_price_from_raydium(
    oracle_account: &AccountInfo
) -> Result<u64, ProgramError> {
    //  Fetch and decode the price data from the oracle account
 
    let data = oracle_account.try_borrow_data()?;
    let price_data = decode_price_data(&data)?;

    Ok(price_data.price)
}


fn decode_price_data(data: &[u8]) -> Result<PriceData, ProgramError> {
    // Ensure the data is at least 16 bytes long to include the timestamp
    if data.len() < 16 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Extract the price from the first 8 bytes
    let price = u64::from_le_bytes(data[0..8].try_into().unwrap());

    // Extract the timestamp from the next 8 bytes
    let timestamp = u64::from_le_bytes(data[8..16].try_into().unwrap());

    Ok(PriceData { price, timestamp })
}

struct PriceData {
    price: u64,
    timestamp: u64,  // Added timestamp to the PriceData struct
}

pub fn swap_via_raydium(
    accounts: &Context<SwapViaRaydium>, 
    swap_amount: u64
) -> ProgramResult {
    // Assuming `accounts` includes all necessary accounts like user's SOL account, 
    // destination token account, Raydium's program account, etc.

    let ix = spl_token_swap::instruction::swap(
        &raydium_program_id, // Raydium swap program ID
        &accounts.user_sol_account.key, // User's SOL account
        &accounts.pool_sol_account.key, // Raydium pool SOL account
        &accounts.pool_token_account.key, // Raydium pool token account
        &accounts.user_token_account.key, // Destination token account
        &accounts.pool_mint.key, // Pool mint account
        &accounts.fee_account.key, // Fee account (if applicable)
        None, // Host fee account (if applicable)
        swap_amount,
        0 // Minimum amount of tokens to receive (set to 0 for simplicity)
    )?;

    msg!("Calling the Raydium swap program...");
    invoke(
        &ix,
        &[
            accounts.user_sol_account.clone(),
            accounts.pool_sol_account.clone(),
            accounts.pool_token_account.clone(),
            accounts.user_token_account.clone(),
            accounts.pool_mint.clone(),
            accounts.fee_account.clone(),
            accounts.token_program.clone(), // SPL Token program
            accounts.system_program.clone(), // Solana System program
        ],
    )?;

    msg!("Swap completed successfully.");
    Ok(())
}

pub fn increase_liquidity_on_raydium(
    accounts: &Context<IncreaseLiquidityOnRaydium>, 
    liquidity_amount: u64,
    minimum_token_b_amount: u64
) -> ProgramResult {
    // Assuming `accounts` includes all necessary accounts like user's token accounts, 
    // Raydium's pool accounts, etc.

    let ix = raydium_sdk::instruction::add_liquidity(
        &raydium_program_id, // Raydium liquidity program ID
        &accounts.user_token_a_account.key, // User's token A account
        &accounts.user_token_b_account.key, // User's token B account
        &accounts.pool_token_a_account.key, // Raydium pool token A account
        &accounts.pool_token_b_account.key, // Raydium pool token B account
        &accounts.pool_lp_token_account.key, // Raydium pool LP token account
        &accounts.user_lp_token_account.key, // User's LP token account
        liquidity_amount,
        minimum_token_b_amount
    )?;

    msg!("Calling the Raydium add liquidity program...");
    invoke(
        &ix,
        &[
            accounts.user_token_a_account.clone(),
            accounts.user_token_b_account.clone(),
            accounts.pool_token_a_account.clone(),
            accounts.pool_token_b_account.clone(),
            accounts.pool_lp_token_account.clone(),
            accounts.user_lp_token_account.clone(),
            accounts.token_program.clone(), // SPL Token program
            accounts.system_program.clone(), // Solana System program
        ],
    )?;

    msg!("Liquidity addition completed successfully.");
    Ok(())
}

pub fn ensure_liquidity_representation(ctx: Context<IncreaseLiquidityOnRaydium>, target_percentage: u64) -> ProgramResult {
    let total_fund_value = get_total_fund_value(&ctx)?;
    let required_liquidity_value = total_fund_value * target_percentage / 100;

    let current_liquidity_value = get_current_liquidity_value(&ctx)?;
    if current_liquidity_value < required_liquidity_value {
        let difference = required_liquidity_value - current_liquidity_value;
        increase_liquidity_on_raydium(ctx, difference)?;
    }

    Ok(())
}
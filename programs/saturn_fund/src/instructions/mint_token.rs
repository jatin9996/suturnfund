use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, TokenAccount, Transfer};
use solana_program::program::invoke;
use solana_program::program_pack::Pack;
use solana_program::sysvar::rent::Rent;
use solana_program::sysvar::Sysvar;

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub holding_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, token::Token>,
    pub rent: Sysvar<'info, Rent>,
    pub raydium_program_id: AccountInfo<'info>, // Raydium program ID
    pub user_sol_account: AccountInfo<'info>, // User's SOL account
    pub pool_sol_account: AccountInfo<'info>, // Raydium pool SOL account
    pub pool_token_account: AccountInfo<'info>, // Raydium pool token account
    pub pool_mint: AccountInfo<'info>, // Pool mint account
    pub fee_account: AccountInfo<'info>, // Fee account (if applicable)
    pub system_program: Program<'info>, // Solana System program
}

#[derive(Accounts)]
pub struct FetchConfig<'info> {
    #[account]
    pub config: Account<'info, Config>,
}

#[account]
pub struct Config {
    pub target_holding_amount: u64,
}

pub fn handler(ctx: Context<MintToken>, amount: u64) -> ProgramResult {
    let user = &ctx.accounts.user;
    let user_token_account = &ctx.accounts.user_token_account;
    let fund_account = &ctx.accounts.fund_account;
    let holding_account = &ctx.accounts.holding_account;
    let mint = &ctx.accounts.mint;
    let token_program = &ctx.accounts.token_program;

    // Define the seed and nonce for the PDA
    let seeds = &[b"mint_authority", mint.key.as_ref(), &[mint.nonce]];
    let signer = &[&seeds[..]];

    // Step 1: Calculate the amount of $STRN to mint
    let fund_price = get_fund_price(); // Fetch or calculate the current fund price
    let strn_amount = calculate_strn_amount(amount, fund_price)?;

    // Step 2: Mint $STRN to the user
    token::mint_to(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            MintTo {
                mint: mint.to_account_info(),
                to: user_token_account.to_account_info(),
                authority: mint.to_account_info(),
            },
            signer,
        ),
        strn_amount,
    )?;

    // Step 3: Determine fund allocation
    let target_holding_amount = get_target_holding_amount(ctx)?;
    let holding_balance = get_balance(holding_account)?;
    let fund_balance = get_balance(fund_account)?;

    if holding_balance < target_holding_amount {
        let to_holding = std::cmp::min(amount, target_holding_amount - holding_balance);
        let to_fund = amount - to_holding;

        // Step 4a: Transfer to holding account
        transfer_to_account(user, holding_account, to_holding)?;

        // Step 4b: Transfer remaining to fund account
        if to_fund > 0 {
            transfer_to_account(user, fund_account, to_fund)?;
            allocate_into_holdings(fund_account, to_fund)?;
        }
    } else {
        // Step 4c: Transfer directly to fund account
        transfer_to_account(user, fund_account, amount)?;
        allocate_into_holdings(fund_account, amount)?;
    }

    Ok(())
}

fn calculate_strn_amount(amount: u64, fund_price: u64) -> Result<u64, ProgramError> {
    let base_amount = amount * fund_price / 100; // Assuming fund_price is the price of 1 SOL in $STRN
    let min_amount = base_amount * 99 / 100;
    let max_amount = base_amount * 101 / 100;
    Ok(base_amount) // Adjust this logic based on actual requirements
}

pub fn get_target_holding_amount(ctx: Context<FetchConfig>) -> Result<u64, ProgramError> {
    Ok(ctx.accounts.config.target_holding_amount)
}

fn get_balance(account: &AccountInfo) -> Result<u64, ProgramError> {
    // Assuming the account data is packed in a way that we can directly read the balance
    let account_data = account.try_borrow_data()?;
    let balance = u64::from_le_bytes(account_data[..8].try_into().map_err(|_| ProgramError::InvalidAccountData)?);

    Ok(balance)
}

fn transfer_to_account(from: &AccountInfo, to: &AccountInfo, amount: u64) -> ProgramResult {
    let ix = solana_program::system_instruction::transfer(
        from.key,
        to.key,
        amount,
    );

    invoke(
        &ix,
        &[
            from.clone(),
            to.clone(),
        ],
    )
}

fn allocate_into_holdings(fund_account: &AccountInfo, amount: u64) -> ProgramResult {
    // Split the amount into 50% SOL and 50% for swapping
    let sol_amount = amount / 2;
    let swap_amount = amount - sol_amount; // To handle odd amounts

    // Keep 50% of the SOL in the fund account (already done by transferring to fund_account)
    // Swap the remaining 50% via Raydium
    swap_via_raydium(fund_account, swap_amount)?;

    // Increase liquidity on Raydium with the swapped tokens
    increase_liquidity_on_raydium(fund_account, swap_amount)?;

    Ok(())
}

fn swap_via_raydium(ctx: Context<MintToken>, amount: u64) -> ProgramResult {
    let ix = spl_token_swap::instruction::swap(
        &ctx.accounts.raydium_program_id, // Raydium swap program ID
        &ctx.accounts.user_sol_account.key, // User's SOL account
        &ctx.accounts.pool_sol_account.key, // Raydium pool SOL account
        &ctx.accounts.pool_token_account.key, // Raydium pool token account
        &ctx.accounts.user_token_account.key, // Destination token account
        &ctx.accounts.pool_mint.key, // Pool mint account
        &ctx.accounts.fee_account.key, // Fee account (if applicable)
        None, // Host fee account (if applicable)
        amount,
        0 // Minimum amount of tokens to receive (set to 0 for simplicity)
    )?;

    msg!("Calling the Raydium swap program...");
    invoke(
        &ix,
        &[
            ctx.accounts.user_sol_account.clone(),
            ctx.accounts.pool_sol_account.clone(),
            ctx.accounts.pool_token_account.clone(),
            ctx.accounts.user_token_account.clone(),
            ctx.accounts.pool_mint.clone(),
            ctx.accounts.fee_account.clone(),
            ctx.accounts.token_program.clone(), // SPL Token program
            ctx.accounts.system_program.clone(), // Solana System program
        ],
    )?;

    msg!("Swap completed successfully.");
    Ok(())
}

fn increase_liquidity_on_raydium(ctx: Context<MintToken>, amount: u64) -> ProgramResult {
    let ix = raydium_sdk::instruction::add_liquidity(
        &ctx.accounts.raydium_program_id, // Raydium liquidity program ID
        &ctx.accounts.user_token_a_account.key, // User's token A account
        &ctx.accounts.user_token_b_account.key, // User's token B account
        &ctx.accounts.pool_token_a_account.key, // Raydium pool token A account
        &ctx.accounts.pool_token_b_account.key, // Raydium pool token B account
        &ctx.accounts.pool_lp_token_account.key, // Raydium pool LP token account
        &ctx.accounts.user_lp_token_account.key, // User's LP token account
        amount,
        0 // Minimum token B amount (set to 0 for simplicity)
    )?;

    msg!("Calling the Raydium add liquidity program...");
    invoke(
        &ix,
        &[
            ctx.accounts.user_token_a_account.clone(),
            ctx.accounts.user_token_b_account.clone(),
            ctx.accounts.pool_token_a_account.clone(),
            ctx.accounts.pool_token_b_account.clone(),
            ctx.accounts.pool_lp_token_account.clone(),
            ctx.accounts.user_lp_token_account.clone(),
            ctx.accounts.token_program.clone(), // SPL Token program
            ctx.accounts.system_program.clone(), // Solana System program
        ],
    )?;

    msg!("Liquidity addition completed successfully.");
    Ok(())
}
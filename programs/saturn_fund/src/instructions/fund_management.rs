use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct FundManagement<'info> {
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    pub solana_holdings_account: Account<'info, TokenAccount>, // Account holding Solana
    pub token_program: Program<'info, token::Token>,
    pub allocation_pda: Account<'info, Allocation>,
    pub token_accounts: Vec<AccountInfo<'info>>, // Accounts for other tokens held by the fund
    pub solana_mint: AccountInfo<'info>, // Mint for Solana
}

pub fn ensure_solana_balance(ctx: Context<FundManagement>) -> ProgramResult {
    let total_fund_value = get_total_fund_value(&ctx.accounts.fund_account)?;
    let solana_balance = ctx.accounts.fund_account.amount; // Changed to fund_account

    let required_solana_balance = total_fund_value / 2; // 50% of total fund value

    if solana_balance < required_solana_balance {
        let difference = required_solana_balance - solana_balance;
        // Logic to buy or transfer Solana to the fund account
        buy_or_transfer_solana(&ctx, difference)?;
    } else if solana_balance > required_solana_balance {
        let difference = solana_balance - required_solana_balance;
        // Logic to reduce Solana holdings in the fund account
        reduce_solana_holdings(&ctx, difference)?;
    }

    Ok(())
}

fn get_total_fund_value(ctx: &Context<FundManagement>) -> Result<u64, ProgramError> {
    let mut total_value: u64 = 0;

    // Example: Iterate over each token account associated with the fund
    for token_account in &ctx.accounts.token_accounts {
        let market_price = get_market_price(&token_account.mint)?; // Fetch market price for the token
        let value = token_account.amount
            .checked_mul(market_price)
            .ok_or(ProgramError::Overflow)?;

        total_value = total_value
            .checked_add(value)
            .ok_or(ProgramError::Overflow)?;
    }

    // Include any other assets, e.g., Solana directly held in the fund
    let sol_price = get_market_price(&ctx.accounts.solana_mint)?; // Fetch market price for Solana
    let sol_value = ctx.accounts.solana_holdings_account.amount
        .checked_mul(sol_price)
        .ok_or(ProgramError::Overflow)?;

    total_value = total_value
        .checked_add(sol_value)
        .ok_or(ProgramError::Overflow)?;

    Ok(total_value)
}

fn buy_or_transfer_solana(ctx: &Context<FundManagement>, amount: u64) -> ProgramResult {
    // Assuming `ctx.accounts.source_sol_account` is the account from which SOL will be transferred
    // and `ctx.accounts.solana_holdings_account` is the destination account

    let transfer_ix = solana_program::system_instruction::transfer(
        &ctx.accounts.source_sol_account.key(),
        &ctx.accounts.solana_holdings_account.key(),
        amount,
    );

    msg!("Transferring SOL...");
    solana_program::program::invoke(
        &transfer_ix,
        &[
            ctx.accounts.source_sol_account.to_account_info(),
            ctx.accounts.solana_holdings_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    msg!("SOL transferred successfully.");
    Ok(())
}

fn reduce_solana_holdings(ctx: &Context<FundManagement>, amount: u64) -> ProgramResult {
    // Assuming `ctx.accounts.solana_holdings_account` is the account from which SOL will be transferred
    // and `ctx.accounts.destination_account` is the account to receive SOL

    let transfer_ix = solana_program::system_instruction::transfer(
        &ctx.accounts.solana_holdings_account.key(),
        &ctx.accounts.destination_account.key(),
        amount,
    );

    msg!("Transferring SOL out of holdings...");
    solana_program::program::invoke(
        &transfer_ix,
        &[
            ctx.accounts.solana_holdings_account.to_account_info(),
            ctx.accounts.destination_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    msg!("SOL transfer completed successfully.");
    Ok(())
}

/// Adjusts fund holdings based on the current allocation without minting tokens
pub fn adjust_fund_holdings(ctx: &Context<FundManagement>, allocations: Vec<(Pubkey, u64)>) -> ProgramResult {
    let total_fund_value = get_total_fund_value(ctx)?;

    for (mint, percentage) in allocations {
        let target_value = total_fund_value * percentage / 100;
        let current_account = find_account_by_mint(&ctx.accounts.token_accounts, &mint)?;
        let current_value = current_account.amount;

        if current_value < target_value {
            // Transfer tokens to this account to increase holdings
            let amount_needed = target_value - current_value;
            transfer_tokens_to_account(ctx, &current_account, amount_needed)?;
        } else if current_value > target_value {
            // Transfer tokens from this account to decrease holdings
            let excess_amount = current_value - target_value;
            transfer_tokens_from_account(ctx, &current_account, excess_amount)?;
        }
    }

    Ok(())
}

fn find_account_by_mint(accounts: &[AccountInfo], mint: &Pubkey) -> Result<AccountInfo, ProgramError> {
    accounts.iter().find(|account| account.mint == *mint).ok_or(ProgramError::AccountNotFound)
}

fn transfer_tokens_to_account(ctx: &Context<FundManagement>, account: &AccountInfo, amount: u64) -> ProgramResult {
    let transfer_ix = spl_token::instruction::transfer(
        &ctx.accounts.token_program.key(),
        &ctx.accounts.source_token_account.key(),
        &account.key(),
        &ctx.accounts.authority.key(),
        &[&ctx.accounts.authority.key()],
        amount,
    )?;

    msg!("Transferring tokens to account...");
    solana_program::program::invoke_signed(
        &transfer_ix,
        &[
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.source_token_account.to_account_info(),
            account.to_account_info(),
            ctx.accounts.authority.to_account_info(),
        ],
        &[],
    )?;

    msg!("Tokens transferred successfully.");
    Ok(())
}

fn transfer_tokens_from_account(ctx: &Context<FundManagement>, account: &AccountInfo, amount: u64) -> ProgramResult {
    let transfer_ix = spl_token::instruction::transfer(
        &ctx.accounts.token_program.key(),
        &account.key(),
        &ctx.accounts.destination_token_account.key(),
        &ctx.accounts.authority.key(),
        &[&ctx.accounts.authority.key()],
        amount,
    )?;

    msg!("Transferring tokens from account...");
    solana_program::program::invoke_signed(
        &transfer_ix,
        &[
            ctx.accounts.token_program.to_account_info(),
            account.to_account_info(),
            ctx.accounts.destination_token_account.to_account_info(),
            ctx.accounts.authority.to_account_info(),
        ],
        &[],
    )?;

    msg!("Tokens transferred successfully.");
    Ok(())
}
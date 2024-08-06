use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ManageTransactionAccount<'info> {
    #[account(mut)]
    pub transaction_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    pub allocation_pda: Account<'info, Allocation>,
    pub token_program: Program<'info, token::Token>,
    pub price_oracle: AccountInfo<'info>, // Account holding price data
}

pub fn manage_transaction_account(ctx: Context<ManageTransactionAccount>) -> ProgramResult {
    let allocation = &ctx.accounts.allocation_pda;
    let fund_value = get_fund_value(&ctx.accounts.fund_account, &ctx.accounts.price_oracle)?;
    let transaction_balance = ctx.accounts.transaction_account.amount;

    let target_balance = fund_value * allocation.target_amount_percentage as u64 / 100;
    let baseline_balance = fund_value * allocation.baseline_amount_percentage as u64 / 100;

    if transaction_balance < baseline_balance {
        let amount_needed = baseline_balance - transaction_balance;
        // Transfer Solana from fund account to transaction account
        transfer_solana(&ctx.accounts.fund_account, &ctx.accounts.transaction_account, amount_needed)?;
    } else if transaction_balance > target_balance {
        let excess_amount = transaction_balance - target_balance;
        // Transfer Solana from transaction account to fund account
        transfer_solana(&ctx.accounts.transaction_account, &ctx.accounts.fund_account, excess_amount)?;
    }

    Ok(())
}

fn get_fund_value(fund_account: &Account<TokenAccount>, price_oracle: &AccountInfo) -> Result<u64, ProgramError> {
    // Calculate the total value of the fund based on its holdings and current market prices
    let mut total_value: u64 = 0;

    for token_account in &fund_account {
        let price_per_token = get_price_from_oracle(price_oracle, &token_account.mint)?;
        let account_value = price_per_token * token_account.amount;
        total_value += account_value;
    }

    Ok(total_value)
}

fn get_price_from_oracle(price_oracle: &AccountInfo, mint: &Pubkey) -> Result<u64, ProgramError> {
    // Fetch and decode the price for the given mint from the oracle account
    let data = price_oracle.try_borrow_data()?;
    let price_data = decode_price_data(&data, mint)?;

    Ok(price_data.price)
}

struct OraclePriceData {
    price: u64, // Price stored in lamports
}

fn decode_price_data(data: &[u8], mint: &Pubkey) -> Result<OraclePriceData, ProgramError> {
    // Check that the data received is at least 8 bytes long, which is needed for a u64
    if data.len() < 8 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Extract the price from the first 8 bytes
    let price_bytes = &data[0..8]; // Get the slice containing the price
    let price = u64::from_le_bytes(price_bytes.try_into().map_err(|_| ProgramError::InvalidAccountData)?);

    Ok(OraclePriceData { price })
}

fn transfer_solana(from: &Account<TokenAccount>, to: &Account<TokenAccount>, amount: u64) -> ProgramResult {
    let ix = solana_program::system_instruction::transfer(
        from.to_account_info().key,
        to.to_account_info().key,
        amount,
    );

    invoke(
        &ix,
        &[
            from.to_account_info(),
            to.to_account_info(),
        ],
    )
}
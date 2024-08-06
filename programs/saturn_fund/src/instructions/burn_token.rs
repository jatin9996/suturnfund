use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, TokenAccount, Transfer};

// Define the context for the BurnToken instruction
#[derive(Accounts)]
pub struct BurnToken<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub holding_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, token::Token>,
    pub allocation_pda: Account<'info, Allocation>,
}
a
pub fn handler(ctx: Context<BurnToken>, amount: u64) -> ProgramResult {
    let user_token_account = &ctx.accounts.user_token_account;
    let holding_account = &ctx.accounts.holding_account;
    let fund_account = &ctx.accounts.fund_account;
    let token_program = &ctx.accounts.token_program;
    let allocation_pda = &ctx.accounts.allocation_pda;

    // Step 1: Receive $STRN tokens from the user
    token::burn(
        CpiContext::new(
            token_program.to_account_info(),
            Burn {
                to: user_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    // Step 2: Calculate the equivalent amount of Solana
    let solana_equivalent = calculate_solana_equivalent(amount)?;

    // Add a check against the baseline amount in the PDA
    let baseline_amount = allocation_pda.baseline_amount_percentage as u64 * get_total_fund_value(&ctx.accounts.fund_account, market_price_per_token)? / 100;
    if holding_account.amount < baseline_amount {
        return Err(ProgramError::InsufficientFunds);
    }

    if holding_account.amount >= solana_equivalent {
        // Step 4a: Transfer Solana to the user
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                Transfer {
                    from: holding_account.to_account_info(),
                    to: user_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            solana_equivalent,
        )?;
    } else {
        // Step 4b: Handle partial liquidation of the fund account
        let required_amount = solana_equivalent - holding_account.amount;
        let half_required_amount = required_amount / 2;

        // Transfer 50% from holding tokens
        liquidate_holding_tokens(ctx, half_required_amount)?;

        // Transfer 50% from Solana
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                Transfer {
                    from: fund_account.to_account_info(),
                    to: holding_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            half_required_amount,
        )?;

        // Transfer the required amount to the user
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                Transfer {
                    from: holding_account.to_account_info(),
                    to: user_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            solana_equivalent,
        )?;

        // Compensate for transaction fees
        let transaction_fee_compensation = calculate_transaction_fee();
        token::transfer(
            CpiContext::new(
                token_program.to_account_info(),
                Transfer {
                    from: fund_account.to_account_info(),
                    to: user_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            transaction_fee_compensation,
        )?;
    }

    // Step 5: Burn the $STRN tokens received from the user
    token::burn(
        CpiContext::new(
            token_program.to_account_info(),
            Burn {
                to: user_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    Ok(())
}

// Helper function to calculate the equivalent amount of Solana
fn calculate_solana_equivalent(amount: u64) -> Result<u64, ProgramError> {
    // Implement the logic to calculate the Solana equivalent based on the fund's price
    let fund_price_per_token = 20; // Example: price per token in cents
    let solana_price = 5000; // Example: price of Solana in cents

    // Calculate the total value in cents
    let total_value_in_cents = amount * fund_price_per_token;

    // Convert the total value in cents to Solana
    let solana_equivalent = total_value_in_cents / solana_price;

    Ok(solana_equivalent)
}

// Helper function to liquidate holding tokens
fn liquidate_holding_tokens(ctx: Context<BurnToken>, amount: u64) -> ProgramResult {
   
    let swap_cpi_accounts = Swap {
        from: ctx.accounts.holding_account.to_account_info(),
        to: ctx.accounts.fund_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let swap_cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info(), swap_cpi_accounts);
    token::swap(swap_cpi_context, amount)?;
    Ok(())
}

// Helper function to calculate transaction fee compensation
fn calculate_transaction_fee() -> u64 {
    
    let base_fee = 5; // Base fee in lamports
    let congestion_level = get_network_congestion(); 
    let transaction_size = get_transaction_size(); 

    
    let congestion_fee = congestion_level * 2;
    let size_fee = transaction_size / 1024; 

    base_fee + congestion_fee + size_fee
}

// Hypothetical helper function to fetch current network congestion level
fn get_network_congestion() -> u64 {
    
    let current_hour = chrono::Local::now().hour(); // Using chrono crate to get current hour

    match current_hour {
        0..=6 => 1, // Low congestion during night hours
        7..=10 | 17..=20 => 3, // High congestion during morning and evening rush hours
        _ => 2, // Medium congestion during other hours
    }
}

// Hypothetical helper function to calculate the transaction size
fn get_transaction_size() -> u64 {
    
    let base_size = 200; // Base size in bytes for a typical transaction
    let dynamic_size = 50; // Additional bytes per operation involved in the transaction

    // Example: Assume 3 operations per transaction
    let operations_count = 3;

    base_size + (dynamic_size * operations_count)
}

// Helper function to get the total value of the fund
fn get_total_fund_value(fund_account: &Account<TokenAccount>, market_price_per_token: u64) -> Result<u64, ProgramError> {
    // Calculate the total value of the fund based on the market price per token
    let total_value = fund_account.amount.checked_mul(market_price_per_token)
        .ok_or(ProgramError::Overflow)?;

    Ok(total_value)
}
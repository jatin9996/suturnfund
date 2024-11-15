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
}

pub fn handler(ctx: Context<BurnToken>, amount: u64) -> ProgramResult {
    let user_token_account = &ctx.accounts.user_token_account;
    let holding_account = &ctx.accounts.holding_account;
    let fund_account = &ctx.accounts.fund_account;
    let token_program = &ctx.accounts.token_program;

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

    // Step 3: Check if the holding account has enough Solana
    if holding_account.amount >= solana_equivalent {
        // Step 4a: Transfer Solana to the user
        token::transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                Transfer {
                    from: holding_account.to_account_info(),
                    to: user_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info().clone(),
                },
                &[&ctx.accounts.program_authority.to_account_info().key],
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
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                Transfer {
                    from: fund_account.to_account_info(),
                    to: holding_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info().clone(),
                },
                &[&ctx.accounts.program_authority.to_account_info().key],
            ),
            half_required_amount,
        )?;

        // Transfer the remaining amount to the user
        token::transfer(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                Transfer {
                    from: holding_account.to_account_info(),
                    to: user_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info(),
                },
                &[&ctx.accounts.program_authority.to_account_info().key],
            ),
            holding_account.amount, // Transfer whatever is left in the holding account
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
    // Assuming `FUND_PRICE_PER_UNIT` is the price of one unit of the fund in terms of Solana
    const FUND_PRICE_PER_UNIT: u64 = 10; 

    let solana_equivalent = amount * FUND_PRICE_PER_UNIT;
    Ok(solana_equivalent)
}

// Helper function to liquidate holding tokens
fn liquidate_holding_tokens(ctx: Context<BurnToken>, amount: u64) -> ProgramResult {
    // Assuming there is a function `swap_tokens_for_solana` that handles the swap
    swap_tokens_for_solana(ctx, amount)?;

    Ok(())
}

// Mock-up of the swap function (you will need to implement this based on your specific requirements)
fn swap_tokens_for_solana(ctx: Context<BurnToken>, amount: u64) -> ProgramResult {
    // Define the DEX program ID
    let dex_program_id = Pubkey::from_str("EnterDEXProgramIDHere").unwrap();

    // Define the market and the open orders accounts
    let market = Pubkey::from_str("EnterMarketPubkeyHere").unwrap();
    let open_orders = Pubkey::from_str("EnterOpenOrdersPubkeyHere").unwrap();

    // Define the source and destination token accounts
    let source_token_account = ctx.accounts.holding_account.to_account_info();
    let destination_token_account = ctx.accounts.user_token_account.to_account_info();

    // Define the user's wallet account, which will be the authority
    let user_wallet = ctx.accounts.user.to_account_info();

    // Create a CPI context for the DEX swap
    let cpi_accounts = Swap {
        market: market,
        open_orders: open_orders,
        source_token_account: source_token_account,
        destination_token_account: destination_token_account,
        user_wallet: user_wallet,
    };
    let cpi_program = CpiProgram::new(&dex_program_id, &cpi_accounts);

    // Assuming `swap` is a method provided by the DEX program
    let swap_instruction = dex::instruction::swap(
        &dex_program_id,
        &market,
        &open_orders,
        &source_token_account.key(),
        &destination_token_account.key(),
        &user_wallet.key(),
        amount,
    )?;

    // Perform the CPI call to the DEX program
    anchor_lang::solana_program::program::invoke(
        &swap_instruction,
        &[
            market.to_account_info(),
            open_orders.to_account_info(),
            source_token_account,
            destination_token_account,
            user_wallet,
        ],
    )?;

    Ok(())
}
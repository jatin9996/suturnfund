use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, TokenAccount, Transfer};
use solana_program::program::invoke;
use solana_program::program_pack::Pack;
use solana_program::sysvar::rent::Rent;
use solana_program::sysvar::Sysvar;

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // User is still the signer to pay for transaction fees
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
    pub config_account: Account<'info, TokenAccount>, // Configuration account
    /// ADD: Program's authority account, possibly a PDA
    #[account(mut)]
    pub program_authority: AccountInfo<'info>,
}

pub fn handler(ctx: Context<MintToken>, amount: u64) -> ProgramResult {
    let user_token_account = &ctx.accounts.user_token_account;
    let fund_account = &ctx.accounts.fund_account;
    let holding_account = &ctx.accounts.holding_account;
    let mint = &ctx.accounts.mint;
    let token_program = &ctx.accounts.token_program;
    let program_authority = &ctx.accounts.program_authority;

    // Step 1: Calculate the amount of $STRN to mint
    let strn_amount = calculate_strn_amount(amount)?;

    // Step 2: Mint $STRN to the user
    token::mint_to(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            MintTo {
                mint: mint.to_account_info(),
                to: user_token_account.to_account_info(),
                authority: program_authority.to_account_info(),
            },
            &[&program_authority.key], // Signer seeds if program_authority is a PDA
        ),
        strn_amount,
    )?;

    // Step 3: Determine fund allocation
    let target_holding_amount = get_target_holding_amount(&ctx)?;
    let holding_balance = get_balance(holding_account)?;
    let fund_balance = get_balance(fund_account)?;

    if holding_balance < target_holding_amount {
        let to_holding = std::cmp::min(amount, target_holding_amount - holding_balance);
        let to_fund = amount - to_holding;

        // Step 4a: Transfer to holding account
        transfer_to_account(program_authority, holding_account, to_holding)?;

        // Step 4b: Transfer remaining to fund account
        if to_fund > 0 {
            transfer_to_account(program_authority, fund_account, to_fund)?;
            allocate_into_holdings(fund_account, to_fund)?;
        }
    } else {
        // Step 4c: Transfer directly to fund account
        transfer_to_account(program_authority, fund_account, amount)?;
        allocate_into_holdings(fund_account, amount)?;
    }

    Ok(())
}

fn calculate_strn_amount(amount: u64) -> Result<u64, ProgramError> {
    // Assume 1 SOL = 100 $STRN for simplicity
    let base_rate: u64 = 100;

    // Calculate the base amount of $STRN
    let base_amount = amount * base_rate;

    // Apply ±1% slippage
    let min_amount = base_amount * 99 / 100;
    let max_amount = base_amount * 101 / 100;

    // For simplicity, we return the base amount here
    // In a real scenario, you might want to return a value within the slippage range
    Ok(base_amount)
}

fn get_target_holding_amount(ctx: &Context<MintToken>) -> Result<u64, ProgramError> {
    // Assuming there's a configuration account that stores target holding amount
    let config_account = &ctx.accounts.config_account; // Add this account to the Context structure

    // Assuming the target holding amount is stored at the beginning of the config account data
    let data = config_account.try_borrow_data()?;
    let target_holding_amount = u64::from_le_bytes(data[..8].try_into().map_err(|_| ProgramError::InvalidAccountData)?);

    Ok(target_holding_amount)
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

fn swap_via_raydium(ctx: &Context<MintToken>, fund_account: &AccountInfo, swap_amount: u64) -> ProgramResult {
    // Define Raydium program ID and swap pool accounts
    let raydium_program_id = Pubkey::from_str("4uQ...RaydiumProgramId...").unwrap();
    let swap_pool_info = AccountInfo::new(
        // Parameters for the swap pool account
        &Pubkey::from_str("Swap...Pool...Pubkey...").unwrap(),
        false,
        true,
        &mut [],
        &mut [],
        &Account::default(),
        false,
        Epoch::default(),
    );
    let swap_authority = Pubkey::from_str("Authority...Pubkey...").unwrap();
    let source_token_account = fund_account.clone(); // Your source token account
    let destination_token_account = fund_account.clone(); // Your destination token account, adjust as necessary

    // Construct the swap instruction
    let swap_instruction = raydium_swap_instruction(
        raydium_program_id,
        swap_pool_info.key,
        swap_authority,
        source_token_account.key,
        destination_token_account.key,
        swap_amount,
    );

    // Invoke the swap instruction
    invoke(
        &swap_instruction,
        &[
            ctx.accounts.token_program.to_account_info(),
            swap_pool_info,
            source_token_account,
            destination_token_account,
        ],
    )
}

fn raydium_swap_instruction(
    program_id: Pubkey,
    swap_pool_pubkey: &Pubkey,
    swap_authority: Pubkey,
    source_token_pubkey: &Pubkey,
    destination_token_pubkey: &Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*swap_pool_pubkey, false),
        AccountMeta::new_readonly(swap_authority, false),
        AccountMeta::new(*source_token_pubkey, false),
        AccountMeta::new(*destination_token_pubkey, false),
    ];

    let data = vec![]; // Populate with actual data required for the swap

    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn increase_liquidity_on_raydium(ctx: &Context<MintToken>, fund_account: &AccountInfo, amount: u64) -> ProgramResult {
    // Define Raydium program ID and liquidity pool accounts
    let raydium_program_id = Pubkey::from_str("RaydiumLiquidityProgramId").unwrap();
    let liquidity_pool_info = AccountInfo::new(
        // Parameters for the liquidity pool account
        &Pubkey::from_str("LiquidityPoolPubkey").unwrap(),
        false,
        true,
        &mut [],
        &mut [],
        &Account::default(),
        false,
        Epoch::default(),
    );
    let liquidity_authority = Pubkey::from_str("LiquidityAuthorityPubkey").unwrap();
    let source_token_account = fund_account.clone(); // Your source token account
    let destination_token_account = fund_account.clone(); // Your destination token account, adjust as necessary

    // Construct the add liquidity instruction
    let add_liquidity_instruction = raydium_add_liquidity_instruction(
        raydium_program_id,
        liquidity_pool_info.key,
        liquidity_authority,
        source_token_account.key,
        destination_token_account.key,
        amount,
    );

    // Invoke the add liquidity instruction
    invoke(
        &add_liquidity_instruction,
        &[
            ctx.accounts.token_program.to_account_info(),
            liquidity_pool_info,
            source_token_account,
            destination_token_account,
        ],
    )
}

fn raydium_add_liquidity_instruction(
    program_id: Pubkey,
    liquidity_pool_pubkey: &Pubkey,
    liquidity_authority: Pubkey,
    source_token_pubkey: &Pubkey,
    destination_token_pubkey: &Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*liquidity_pool_pubkey, false),
        AccountMeta::new_readonly(liquidity_authority, false),
        AccountMeta::new(*source_token_pubkey, false),
        AccountMeta::new(*destination_token_pubkey, false),
    ];

    let data = vec![]; // Populate with actual data required for adding liquidity

    Instruction {
        program_id,
        accounts,
        data,
    }
}
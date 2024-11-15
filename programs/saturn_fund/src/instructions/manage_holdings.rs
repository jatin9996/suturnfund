use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ManageHoldings<'info> {
    #[account(mut)]
    pub fund_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, token::Token>,
    pub token_accounts: Vec<Account<'info, TokenAccount>>, // All token accounts managed by the fund
    // Add other token accounts representing different holdings
}

pub fn rebalance_holdings(ctx: Context<ManageHoldings>) -> ProgramResult {
    // Logic to calculate current percentages of each holding
    let current_percentages = calculate_current_percentages(&ctx)?;

    // Logic to adjust holdings to target percentages
    adjust_to_target_percentages(&ctx, current_percentages)?;

    Ok(())
}

fn calculate_current_percentages(ctx: &Context<ManageHoldings>) -> Result<HashMap<Pubkey, f64>, ProgramError> {
    let total_value = get_total_fund_value(ctx)?;
    let mut percentages = HashMap::new();

    for token_account in &ctx.accounts.token_accounts {
        let market_price = get_market_price(&token_account.mint)?;
        let value = token_account.amount
            .checked_mul(market_price)
            .ok_or(ProgramError::Overflow)?;

        let percentage = (value as f64 / total_value as f64) * 100.0;
        percentages.insert(token_account.mint, percentage);
    }

    Ok(percentages)
}

fn adjust_to_target_percentages(
    ctx: &Context<ManageHoldings>, 
    target_percentages: HashMap<Pubkey, f64>
) -> ProgramResult {
    let total_fund_value = get_total_fund_value(ctx)?;
    let current_percentages = calculate_current_percentages(ctx)?;

    for (mint, target_percentage) in target_percentages {
        let current_percentage = current_percentages.get(&mint).unwrap_or(&0.0);
        let amount_to_adjust = calculate_amount_to_adjust(
            ctx,
            &mint,
            *current_percentage,
            target_percentage,
            total_fund_value
        )?;

        if current_percentage < &target_percentage {
            buy_tokens(ctx, &mint, amount_to_adjust)?;
        } else if current_percentage > &target_percentage {
            sell_tokens(ctx, &mint, amount_to_adjust)?;
        }
    }

    Ok(())
}

fn calculate_amount_to_adjust(
    ctx: &Context<ManageHoldings>,
    mint: &Pubkey,
    current_percentage: f64,
    target_percentage: f64,
    total_fund_value: u64
) -> Result<u64, ProgramError> {
    let market_price = get_market_price(mint)?;
    let current_value = (current_percentage / 100.0) * total_fund_value as f64;
    let target_value = (target_percentage / 100.0) * total_fund_value as f64;

    let amount_to_adjust = ((target_value - current_value) / market_price).abs();

    Ok(amount_to_adjust as u64)
}

fn buy_tokens(ctx: &Context<ManageHoldings>, mint: &Pubkey, amount: u64) -> ProgramResult {
    // Assuming you have a DEX program and accounts set up for trading
    let dex_program_id = Pubkey::from_str("EnterYourDexProgramIdHere").unwrap();
    let orderbook_account = find_orderbook_account(ctx, mint)?;
    let source_token_account = &ctx.accounts.fund_account; // Your fund's SOL or stablecoin account
    let destination_token_account = find_destination_token_account(ctx, mint)?;

    // Construct the buy order instruction
    let buy_order_ix = construct_buy_order_instruction(
        &dex_program_id,
        &source_token_account.key(),
        &destination_token_account.key(),
        &orderbook_account.key(),
        amount,
        serum_dex::matching::Side::Bid,
        0, // Limit price (0 for market order)
        amount,
        0, // Max native PC quantity including fees (0 for unlimited)
        serum_dex::instruction::SelfTradeBehavior::DecrementTake,
    )?;

    // Invoke the DEX program to execute the buy order
    msg!("Executing buy order on DEX...");
    solana_program::program::invoke(
        &buy_order_ix,
        &[
            ctx.accounts.token_program.to_account_info(),
            source_token_account.to_account_info(),
            destination_token_account.to_account_info(),
            orderbook_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    msg!("Buy order executed successfully.");
    Ok(())
}

fn find_orderbook_account(ctx: &Context<ManageHoldings>, mint: &Pubkey) -> Result<Option<AccountInfo>, ProgramError> {
    // map of market data that includes orderbook accounts
    let markets = get_markets_data(); // This function should fetch or have access to market data

    if let Some(market) = markets.iter().find(|market| &market.base_mint == mint || &market.quote_mint == mint) {
        if let Some(orderbook_account_info) = ctx.accounts.token_accounts.iter().find(|account| &account.key() == &market.orderbook) {
            return Ok(Some(orderbook_account_info.clone()));
        }
    }
    Ok(None) // Return None if no orderbook account is found
}

struct Market {
    base_mint: Pubkey,
    quote_mint: Pubkey,
    orderbook: Pubkey,
}

fn get_markets_data() -> Vec<Market> {
    vec![
        Market {
            base_mint: Pubkey::from_str("BaseMintPubkeyHere").unwrap(),
            quote_mint: Pubkey::from_str("QuoteMintPubkeyHere").unwrap(),
            orderbook: Pubkey::from_str("OrderbookPubkeyHere").unwrap(),
        },
        
    ]
}

fn find_destination_token_account(ctx: &Context<ManageHoldings>, mint: &Pubkey) -> Result<AccountInfo, ProgramError> {
    // Iterate over the token accounts stored in the context to find the one matching the given mint
    ctx.accounts.token_accounts.iter()
        .find(|account| &account.mint == mint)
        .ok_or(ProgramError::AccountNotFound)
        .map(|account| account.to_account_info())
}

fn construct_buy_order_instruction(
    dex_program_id: &Pubkey,
    source_account_pubkey: &Pubkey,
    destination_account_pubkey: &Pubkey,
    orderbook_account_pubkey: &Pubkey,
    amount: u64,
    side: serum_dex::matching::Side,
    limit_price: u64,
    max_coin_qty: u64,
    max_native_pc_qty_including_fees: u64,
    self_trade_behavior: serum_dex::instruction::SelfTradeBehavior
) -> Result<Instruction, ProgramError> {
    let data = serum_dex::instruction::NewOrderV3 {
        side,
        limit_price,
        max_coin_qty,
        max_native_pc_qty_including_fees,
        self_trade_behavior,
        order_type: serum_dex::matching::OrderType::Limit,
        client_order_id: 0,
        limit: 65535,
    };

    let accounts = vec![
        AccountMeta::new_readonly(*dex_program_id, false),
        AccountMeta::new(*source_account_pubkey, true), // Source account (payer
        AccountMeta::new(*orderbook_account_pubkey, false), // Orderbook account
        AccountMeta::new(*destination_account_pubkey, false), // Destination account (to receive tokens)
        AccountMeta::new_readonly(spl_token::id(), false), // SPL Token program
        AccountMeta::new_readonly(sysvar::rent::id(), false), // Rent sysvar
        AccountMeta::new_readonly(sysvar::clock::id(), false), // Clock sysvar
        
    ];

    let instruction = Instruction {
        program_id: *dex_program_id,
        accounts,
        data: serum_dex::instruction::MarketInstruction::NewOrderV3(data).pack(),
    };

    Ok(instruction)
}

pub fn sell_tokens(ctx: &Context<ManageHoldings>, mint: &Pubkey, amount: u64) -> ProgramResult {
    // Assuming you have a DEX program and accounts set up for trading
    let dex_program_id = Pubkey::from_str("EnterYourDexProgramIdHere").unwrap();
    let orderbook_account = find_orderbook_account(ctx, mint)?;
    let source_token_account = find_source_token_account(ctx, mint)?; // The account holding the tokens to sell
    let destination_token_account = find_destination_token_account(ctx, mint)?; // Typically a USD or stablecoin account

    // Construct the sell order instruction
    let sell_order_ix = construct_sell_order_instruction(
        &dex_program_id,
        &source_token_account.key(),
        &destination_token_account.key(),
        &orderbook_account.key(),
        amount,
        serum_dex::matching::Side::Ask, // Selling tokens
        0, // Limit price (0 for market order)
        amount,
        0, // Max native PC quantity including fees (0 for unlimited)
        serum_dex::instruction::SelfTradeBehavior::DecrementTake,
    );

    // Invoke the DEX program to execute the sell order
    msg!("Executing sell order on DEX...");
    solana_program::program::invoke(
        &sell_order_ix,
        &[
            ctx.accounts.token_program.to_account_info(),
            source_token_account.to_account_info(),
            destination_token_account.to_account_info(),
            orderbook_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    msg!("Sell order executed successfully.");
    Ok(())
}

fn construct_sell_order_instruction(
    dex_program_id: &Pubkey,
    source_account_pubkey: &Pubkey,
    destination_account_pubkey: &Pubkey,
    orderbook_account_pubkey: &Pubkey,
    amount: u64,
    side: serum_dex::matching::Side,
    limit_price: u64,
    max_coin_qty: u64,
    max_native_pc_qty_including_fees: u64,
    self_trade_behavior: serum_dex::instruction::SelfTradeBehavior
) -> Instruction {
    // Constructing a sell order similar to the buy order but with the Side::Ask
    let data = serum_dex::instruction::NewOrderV3 {
        side,
        limit_price,
        max_coin_qty,
        max_native_pc_qty_including_fees,
        self_trade_behavior,
        order_type: serum_dex::matching::OrderType::Limit,
        client_order_id: 0,
        limit: 65535,
    };

    let accounts = vec![
        AccountMeta::new_readonly(*dex_program_id, false),
        AccountMeta::new(*source_account_pubkey, true), // Source account (payer)
        AccountMeta::new(*orderbook_account_pubkey, false), // Orderbook account
        AccountMeta::new(*destination_account_pubkey, false), // Destination account (to receive currency)
        AccountMeta::new_readonly(spl_token::id(), false), // SPL Token program
        AccountMeta::new_readonly(sysvar::rent::id(), false), // Rent sysvar
        AccountMeta::new_readonly(sysvar::clock::id(), false), // Clock sysvar
        // Add other necessary accounts like market, open orders, etc.
    ];

    let instruction = Instruction {
        program_id: *dex_program_id,
        accounts,
        data: serum_dex::instruction::MarketInstruction::NewOrderV3(data).pack(),
    };

    Ok(instruction)
}

fn find_source_token_account(ctx: &Context<ManageHoldings>, mint: &Pubkey) -> Result<AccountInfo, ProgramError> {
    // This function finds the source token account for selling tokens
    // It should match the mint of the token you want to sell
    ctx.accounts.token_accounts.iter()
        .find(|account| &account.mint == mint)
        .ok_or(ProgramError::AccountNotFound)
        .map(|account| account.to_account_info())
}

fn find_destination_token_account(ctx: &Context<ManageHoldings>, mint: &Pubkey) -> Result<AccountInfo, ProgramError> {
    // This function finds the destination token account for buying tokens
    // It should match the mint of the token you want to buy
    ctx.accounts.token_accounts.iter()
        .find(|account| &account.mint == mint)
        .ok_or(ProgramError::AccountNotFound)
        .map(|account| account.to_account_info())
}

pub fn distribute_holdings_evenly(ctx: Context<ManageHoldings>) -> ProgramResult {
    let total_fund_value = get_total_fund_value(&ctx)?;
    let target_value_per_holding = (total_fund_value / 2) / 25; // Assuming 25 different holdings for simplicity

    for token_account in &ctx.accounts.token_accounts {
        let current_value = get_market_value(&token_account)?;
        if current_value < target_value_per_holding {
            let amount_needed = target_value_per_holding - current_value;
            // Function to buy or transfer tokens to reach the target value
            buy_or_transfer_tokens(&ctx, &token_account.mint, amount_needed)?;
        }
    }

    Ok(())
}
use anchor_lang::prelude::*;
use crate::state::allocation::Allocation;

#[derive(Accounts)]
pub struct UpdateAllocationPda<'info> {
    #[account(mut, has_one = owner)]
    pub allocation_pda: Account<'info, Allocation>,
    pub owner: Signer<'info>,
}

pub fn handler(ctx: Context<UpdateAllocationPda>, new_allocation: Allocation) -> ProgramResult {
    let allocation_pda = &mut ctx.accounts.allocation_pda;

    // Update the allocation details
    allocation_pda.holding_tokens = new_allocation.holding_tokens;
    allocation_pda.target_amount_percentage = new_allocation.target_amount_percentage;
    allocation_pda.baseline_amount_percentage = new_allocation.baseline_amount_percentage;
    allocation_pda.liquidity_pool_reward_percentage = new_allocation.liquidity_pool_reward_percentage;
    allocation_pda.liquidity_pool_reward_destination = new_allocation.liquidity_pool_reward_destination;

    Ok(())
}

pub fn update_allocation_based_on_market(ctx: Context<UpdateAllocationPda>, market_data: MarketData) -> ProgramResult {
    let allocation_pda = &mut ctx.accounts.allocation_pda;

    //  logic to update allocation based on market data
    allocation_pda.target_amount_percentage = calculate_new_target_percentage(&market_data);
    allocation_pda.baseline_amount_percentage = calculate_new_baseline_percentage(&market_data);
    allocation_pda.liquidity_pool_reward_percentage = calculate_new_reward_percentage(&market_data);

    Ok(())
}

// Helper functions to calculate new percentages based on market data
fn calculate_new_target_percentage(market_data: &MarketData) -> u8 {
    //  logic: Adjust target percentage based on price change and volatility
    let base_percentage = 50; // Base target percentage
    let adjustment_factor = if market_data.price_change_percentage > 5.0 {
        // If price increased by more than 5%, decrease target percentage
        -5
    } else if market_data.price_change_percentage < -5.0 {
        // If price decreased by more than 5%, increase target percentage
        5
    } else {
        // Minimal or no change in price
        0
    };

    let volatility_adjustment = if market_data.volatility_index > 50.0 {
        // High volatility, decrease target percentage
        -5
    } else {
        // Low volatility, no change
        0
    };

    // Calculate final target percentage ensuring it remains within 0-100 bounds
    let final_percentage = (base_percentage as i32 + adjustment_factor + volatility_adjustment).max(0).min(100) as u8;
    final_percentage
}

fn calculate_new_baseline_percentage(market_data: &MarketData) -> u8 {
    //  logic: Adjust baseline percentage based on economic stability
    let base_percentage = 30; // Base baseline percentage
    let stability_adjustment = if market_data.economic_stability_index > 70 {
        // High economic stability
        10
    } else if market_data.economic_stability_index < 30 {
        // Low economic stability
        -10
    } else {
        // Moderate stability
        0
    };

    // Calculate final baseline percentage ensuring it remains within 0-100 bounds
    let final_percentage = (base_percentage as i32 + stability_adjustment).max(0).min(100) as u8;
    final_percentage
}

fn calculate_new_reward_percentage(market_data: &MarketData) -> u8 {
    //  logic: Adjust reward percentage based on market performance
    let base_percentage = 20; // Base reward percentage
    let performance_adjustment = if market_data.performance_index > 80 {
        // Excellent market performance
        5
    } else if market_data.performance_index < 50 {
        // Poor market performance
        -5
    } else {
        // Average market performance
        0
    };

    // Calculate final reward percentage ensuring it remains within 0-100 bounds
    let final_percentage = (base_percentage as i32 + performance_adjustment).max(0).min(100) as u8;
    final_percentage
}
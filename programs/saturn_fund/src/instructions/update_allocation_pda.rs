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

    // Example validation: Ensure percentages do not exceed 100%
    if new_allocation.target_amount_percentage + new_allocation.baseline_amount_percentage > 100 {
        return Err(ProgramError::Custom(ErrorCode::InvalidPercentage as u32));
    }

    // Update the allocation details
    allocation_pda.holding_tokens = new_allocation.holding_tokens;
    allocation_pda.target_amount_percentage = new_allocation.target_amount_percentage;
    allocation_pda.baseline_amount_percentage = new_allocation.baseline_amount_percentage;
    allocation_pda.liquidity_pool_reward_percentage = new_allocation.liquidity_pool_reward_percentage;
    allocation_pda.liquidity_pool_reward_destination = new_allocation.liquidity_pool_reward_destination;

    Ok(())
}
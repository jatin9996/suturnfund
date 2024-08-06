use anchor_lang::prelude::*;

#[account]
pub struct Allocation {
    pub holding_tokens: Vec<TokenAllocation>,
    pub target_amount_percentage: u8,
    pub baseline_amount_percentage: u8,
    pub liquidity_pool_reward_percentage: u8,
    pub liquidity_pool_reward_destination: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TokenAllocation {
    pub token_mint: Pubkey,
    pub percentage: u8,
}

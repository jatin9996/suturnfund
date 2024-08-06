use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CalculatePriceOfFund<'info> {
    pub fund_account: AccountInfo<'info>,
    pub mint_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintToken<'info> {
    pub user_account: AccountInfo<'info>,
    pub fund_account: AccountInfo<'info>,
    pub mint_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub allocation_pda: AccountInfo<'info>,
    pub transaction_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnToken<'info> {
    pub user_account: AccountInfo<'info>,
    pub fund_account: AccountInfo<'info>,
    pub mint_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub allocation_pda: AccountInfo<'info>,
    pub transaction_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CollectLiquidityPoolRewards<'info> {
    pub fund_account: AccountInfo<'info>,
    pub allocation_pda: AccountInfo<'info>,
    pub liquidity_pool_reward_destination: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAllocationPda<'info> {
    pub allocation_pda: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

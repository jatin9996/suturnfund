use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod saturn_fund {
    use super::*;




}

mod instructions;

use anchor_lang::solana_program::{
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    account_info::AccountInfo,
};

use crate::instructions::{
    manage_holdings::sell_tokens,
    mint_management::create_mint,
    fund_management::ensure_solana_balance,
    burn_token::handler as burn_token_handler,
    mint_token::handler as mint_token_handler,
    calculate_price_of_fund::handler as calculate_price_handler,
};

use solana_program::entrypoint;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data[0] {
        0 => mint_token_handler(program_id, accounts, instruction_data),
        1 => burn_token_handler(program_id, accounts, instruction_data),
        2 => ensure_solana_balance(program_id, accounts, instruction_data),
        3 => sell_tokens(program_id, accounts, instruction_data),
        4 => calculate_price_handler(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
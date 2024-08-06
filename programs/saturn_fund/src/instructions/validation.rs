use anchor_lang::prelude::*;

pub fn validate_allocation_percentages(allocation: &Allocation) -> Result<(), ProgramError> {
    if allocation.target_amount_percentage + allocation.baseline_amount_percentage > 100 {
        Err(ProgramError::Custom(ErrorCode::InvalidPercentage as u32))
    } else {
        Ok(())
    }
}

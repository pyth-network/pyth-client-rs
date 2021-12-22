//! Program instruction processor

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    log::{sol_log_compute_units, sol_log_params, sol_log_slice},
    msg,
    pubkey::Pubkey,
};
use crate::{
    instruction::PythClientInstruction,
    PriceConf,
};
use borsh::BorshDeserialize;

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = PythClientInstruction::try_from_slice(input).unwrap();
    match instruction {
        PythClientInstruction::Divide { numerator, denominator } => {
            msg!("Calculating numerator.div(denominator)");
            sol_log_compute_units();
            let result = numerator.div(&denominator);
            sol_log_compute_units();
            msg!("{:?}", result);
            Ok(())
        }
        PythClientInstruction::Noop => {
            msg!("Do nothing");
            msg!("{}", 0_u64);
            Ok(())
        }
    }
}

//! Program instruction processor for end-to-end testing and instruction counts

use borsh::BorshDeserialize;
use solana_program::{
  account_info::AccountInfo,
  entrypoint::ProgramResult,
  pubkey::Pubkey,
  program_error::ProgramError
};

use crate::{
  instruction::PythClientInstruction, load_price, PriceStatus,
};

pub fn process_instruction(
  _program_id: &Pubkey,
  _accounts: &[AccountInfo],
  input: &[u8],
) -> ProgramResult {
  let instruction = PythClientInstruction::try_from_slice(input).unwrap();
  match instruction {
    PythClientInstruction::Divide { numerator, denominator } => {
      numerator.div(&denominator);
      Ok(())
    }
    PythClientInstruction::Multiply { x, y } => {
      x.mul(&y);
      Ok(())
    }
    PythClientInstruction::Add { x, y } => {
      x.add(&y);
      Ok(())
    }
    PythClientInstruction::Normalize { x } => {
      x.normalize();
      Ok(())
    }
    PythClientInstruction::ScaleToExponent { x, expo } => {
      x.scale_to_exponent(expo);
      Ok(())
    }
    PythClientInstruction::Noop => {
      Ok(())
    }
    PythClientInstruction::PriceNotStale { price_account_data } => {
      let price = load_price(&price_account_data[..])?;
      
      match price.get_current_status() {
        PriceStatus::Trading => {
          Ok(())
        }
        _ => Err(ProgramError::Custom(0))
      }
    }
  }
}

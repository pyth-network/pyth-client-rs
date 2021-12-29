use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by Pyth.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum PythError {
  // 0
  /// Invalid instruction data passed in.
  #[error("Failed to unpack instruction data")]
  InvalidAccountData,
  /// Invalid instruction data passed in.
  #[error("Failed to unpack instruction data")]
  BadVersionNumber,
  /// Invalid instruction data passed in.
  #[error("Failed to unpack instruction data")]
  WrongAccountType,
}

impl From<PythError> for ProgramError {
  fn from(e: PythError) -> Self {
    ProgramError::Custom(e as u32)
  }
}

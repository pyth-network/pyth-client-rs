//! Program instructions for end-to-end testing and instruction counts

use crate::PriceStatus;

use {
  crate::id,
  borsh::{BorshDeserialize, BorshSerialize},
  solana_program::instruction::Instruction,
  crate::PriceConf,
};

/// Instructions supported by the pyth-client program, used for testing and
/// instruction counts
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum PythClientInstruction {
  Divide {
    numerator: PriceConf,
    denominator: PriceConf,
  },
  Multiply {
    x: PriceConf,
    y: PriceConf,
  },
  Add {
    x: PriceConf,
    y: PriceConf,
  },
  ScaleToExponent {
    x: PriceConf,
    expo: i32,
  },
  Normalize {
    x: PriceConf,
  },
  /// Don't do anything for comparison
  ///
  /// No accounts required for this instruction
  Noop,

  PriceStatusCheck {
    // Adding Borsh SerDe can be expensive (Price Account is big) and requires to add Borsh SerDe, Debug, Default to all structs
    // (also in enum Default is experimental which we should provide in another way). I think it's best to left structs intact for this purpose.
    price_account_data: Vec<u8>,  
    expected_price_status: PriceStatus
  }
}

pub fn divide(numerator: PriceConf, denominator: PriceConf) -> Instruction {
  Instruction {
    program_id: id(),
    accounts: vec![],
    data: PythClientInstruction::Divide { numerator, denominator }
      .try_to_vec()
      .unwrap(),
  }
}

pub fn multiply(x: PriceConf, y: PriceConf) -> Instruction {
  Instruction {
    program_id: id(),
    accounts: vec![],
    data: PythClientInstruction::Multiply { x, y }
      .try_to_vec()
      .unwrap(),
  }
}

pub fn add(x: PriceConf, y: PriceConf) -> Instruction {
  Instruction {
    program_id: id(),
    accounts: vec![],
    data: PythClientInstruction::Add { x, y }
      .try_to_vec()
      .unwrap(),
  }
}

pub fn scale_to_exponent(x: PriceConf, expo: i32) -> Instruction {
  Instruction {
    program_id: id(),
    accounts: vec![],
    data: PythClientInstruction::ScaleToExponent { x, expo }
      .try_to_vec()
      .unwrap(),
  }
}

pub fn normalize(x: PriceConf) -> Instruction {
  Instruction {
    program_id: id(),
    accounts: vec![],
    data: PythClientInstruction::Normalize { x }
      .try_to_vec()
      .unwrap(),
  }
}

/// Noop instruction for comparison purposes
pub fn noop() -> Instruction {
  Instruction {
    program_id: id(),
    accounts: vec![],
    data: PythClientInstruction::Noop.try_to_vec().unwrap(),
  }
}

// Returns ok if price is not stale
pub fn price_status_check(price_account_data: Vec<u8>, expected_price_status: PriceStatus) -> Instruction {
  Instruction {
    program_id: id(), 
    accounts: vec![],
    data: PythClientInstruction::PriceStatusCheck { price_account_data, expected_price_status }
      .try_to_vec()
      .unwrap(),
  }
}

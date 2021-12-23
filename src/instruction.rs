//! Program instructions, used for end-to-end testing and instruction counts

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
    /// Don't do anything for comparison
    ///
    /// No accounts required for this instruction
    Noop,
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

/// Noop instruction for comparison purposes
pub fn noop() -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::Noop.try_to_vec().unwrap(),
    }
}

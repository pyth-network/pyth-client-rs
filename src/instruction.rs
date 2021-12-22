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

/*
/// Create SquareRoot instruction
pub fn sqrt_u64(radicand: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::SquareRootU64 { radicand }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create SquareRoot instruction
pub fn sqrt_u128(radicand: u128) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::SquareRootU128 { radicand }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn u64_multiply(multiplicand: u64, multiplier: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::U64Multiply {
            multiplicand,
            multiplier,
        }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn u64_divide(dividend: u64, divisor: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::U64Divide { dividend, divisor }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn f32_multiply(multiplicand: f32, multiplier: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::F32Multiply {
            multiplicand,
            multiplier,
        }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn f32_divide(dividend: f32, divisor: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::F32Divide { dividend, divisor }
            .try_to_vec()
            .unwrap(),
    }
}

 */

/// Create PreciseSquareRoot instruction
pub fn noop() -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: PythClientInstruction::Noop.try_to_vec().unwrap(),
    }
}

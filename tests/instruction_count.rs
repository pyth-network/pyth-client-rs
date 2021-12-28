use {
    pyth_client::{id, instruction, PriceConf},
    pyth_client::processor::process_instruction,
    solana_program::{
        instruction::Instruction,
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
};

async fn test_instr(instr: Instruction) {
    let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
        "pyth_client",
        id(),
        processor!(process_instruction),
    )
        .start()
        .await;
    let mut transaction = Transaction::new_with_payer(
        &[instr],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

fn pc(price: i64, conf: u64, expo: i32) -> PriceConf {
    PriceConf {
        price: price,
        conf: conf,
        expo: expo,
    }
}

#[tokio::test]
async fn test_noop() {
    test_instr(instruction::noop()).await;
}

#[tokio::test]
async fn test_scale_to_exponent_down() {
    test_instr(instruction::scale_to_exponent(pc(1, u64::MAX, -1000), 1000)).await
}

#[tokio::test]
async fn test_scale_to_exponent_up() {
    test_instr(instruction::scale_to_exponent(pc(1, u64::MAX, 1000), -1000)).await
}

#[tokio::test]
async fn test_scale_to_exponent_best_case() {
    test_instr(instruction::scale_to_exponent(pc(1, u64::MAX, 10), 10)).await
}

#[tokio::test]
async fn test_normalize_max_conf() {
    test_instr(instruction::normalize(pc(1, u64::MAX, 0))).await
}

#[tokio::test]
async fn test_normalize_max_price() {
    test_instr(instruction::normalize(pc(i64::MAX, 1, 0))).await
}

#[tokio::test]
async fn test_normalize_min_price() {
    test_instr(instruction::normalize(pc(i64::MIN, 1, 0))).await
}

#[tokio::test]
async fn test_normalize_best_case() {
    test_instr(instruction::normalize(pc(1, 1, 0))).await
}

#[tokio::test]
async fn test_div_max_price() {
    test_instr(instruction::divide(
        pc(i64::MAX, 1, 0),
        pc(1, 1, 0)
    )).await;
}

#[tokio::test]
async fn test_div_max_price_2() {
    test_instr(instruction::divide(
        pc(i64::MAX, 1, 0),
        pc(i64::MAX, 1, 0)
    )).await;
}

#[tokio::test]
async fn test_mul_max_price() {
    test_instr(instruction::multiply(
        pc(i64::MAX, 1, 2),
        pc(123, 1, 2),
    )).await;
}

#[tokio::test]
async fn test_mul_max_price_2() {
    test_instr(instruction::multiply(
        pc(i64::MAX, 1, 2),
        pc(i64::MAX, 1, 2),
    )).await;
}

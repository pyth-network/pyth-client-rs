use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction},
    pyth_client::processor::process_instruction,
    std::str::FromStr,
    borsh::BorshDeserialize,
    pyth_client::{id, instruction, PriceConf},
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

#[tokio::test]
async fn test_noop() {
    test_instr(instruction::noop()).await;
}

#[tokio::test]
async fn test_div() {
    test_instr(instruction::divide(
        PriceConf {
            price: 1,
            conf: 1,
            expo: 0
        },
        PriceConf {
            price: 1,
            conf: 1,
            expo: 0
        }
    )).await;
}
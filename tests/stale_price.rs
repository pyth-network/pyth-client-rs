#![cfg(feature = "test-bpf")] // This only runs on bpf

use {
    bytemuck::bytes_of,
    pyth_client::{id, MAGIC, VERSION_2, instruction, PriceType, PriceAccountData, AccountType, AccKey, Ema, PriceComp, PriceInfo, CorpAction, PriceStatus},
    pyth_client::processor::process_instruction,
    solana_program::instruction::Instruction,
    solana_program_test::*,
    solana_sdk::{signature::Signer, transaction::Transaction, transport::TransportError},
};

async fn test_instr(instr: Instruction) -> Result<(), TransportError> {
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
    banks_client.process_transaction(transaction).await
}

fn price_all_zero() -> PriceAccountData {
    let acc_key = AccKey {
        val: [0; 32]
    };

    let ema = Ema {
        val: 0,
        numer: 0,
        denom: 0
    };

    let price_info = PriceInfo {
        conf: 0,
        corp_act: CorpAction::NoCorpAct,
        price: 0,
        pub_slot: 0,
        status: PriceStatus::Unknown
    };

    let price_comp = PriceComp {
        agg: price_info,
        latest: price_info,
        publisher: acc_key
    };

    PriceAccountData {
        magic: MAGIC,
        ver: VERSION_2,
        atype: AccountType::Price as u32,
        size: 0,
        ptype: PriceType::Price,
        expo: 0,
        num: 0,
        num_qt: 0,
        last_slot: 0,
        valid_slot: 0,
        twap: ema,
        twac: ema,
        drv1: 0,
        drv2: 0,
        prod: acc_key,
        next: acc_key,
        prev_slot: 0,
        prev_price: 0,
        prev_conf: 0,
        drv3: 0,
        agg: price_info,
        comp: [price_comp; 32]
    }
}


#[tokio::test]
async fn test_price_not_stale() {
    let mut price = price_all_zero();
    price.agg.status = PriceStatus::Trading;
    test_instr(instruction::price_not_stale(bytes_of(&price).to_vec())).await.unwrap();
}


#[tokio::test]
async fn test_price_stale() {
    let mut price = price_all_zero();
    price.agg.status = PriceStatus::Trading;
    price.agg.pub_slot = 100; // It will cause an overflow because this is bigger than Solana slot which is impossible in reality
    test_instr(instruction::price_not_stale(bytes_of(&price).to_vec())).await.unwrap_err();
}
mod cli;
mod payment_engine;

use std::fs::File;
use std::io;

use anyhow::{Result, anyhow};
use clap::Parser;
use csv::{Reader, WriterBuilder};
use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::warn;

use crate::cli::Cli;
use crate::payment_engine::{PaymentEngine, Transaction, TransactionType};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(git_hash) => git_hash,
    None => "0000000000000000000000000000000000000000",
};
const BUILD_TIMESTAMP: &str = env!("GIT_TIMESTAMP");

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let mut payment_engine = PaymentEngine::new();

    let file = File::open(cli.transactions_file)?;
    let mut reader = Reader::from_reader(file);
    let records = reader.deserialize::<CsvTransaction>();
    for transaction in records {
        let transaction: Transaction = transaction?.try_into()?;
        let transaction_id = transaction.id;

        if let Err(err) = payment_engine.process_transaction(transaction) {
            warn!(transaction_id, ?err, "transaction processing failed");
        }
    }

    let accounts = payment_engine.get_accounts_statuses();

    let mut writer = WriterBuilder::new().from_writer(io::stdout());
    for account in accounts {
        writer.serialize(&account)?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CsvTransaction {
    #[serde(rename = "type")]
    pub transaction_type: CsvTransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CsvTransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TryFrom<CsvTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(value: CsvTransaction) -> Result<Self, Self::Error> {
        let transaction_type = match value.transaction_type {
            CsvTransactionType::Deposit => TransactionType::Deposit {
                amount: value
                    .amount
                    .ok_or_else(|| anyhow!("amount is required for deposit"))?,
            },
            CsvTransactionType::Withdrawal => TransactionType::Withdrawal {
                amount: value
                    .amount
                    .ok_or_else(|| anyhow!("amount is required for withdrawal"))?,
            },
            CsvTransactionType::Dispute => TransactionType::Dispute,
            CsvTransactionType::Resolve => TransactionType::Resolve,
            CsvTransactionType::Chargeback => TransactionType::Chargeback,
        };

        Ok(Transaction::new(value.client, value.tx, transaction_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::ReaderBuilder;
    use rust_decimal::dec;

    #[test]
    fn test_deserialize_deposit() {
        let data = "type,client,tx,amount\n\
                    deposit,1,1001,42.5\n";
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data.as_bytes());
        let mut iter = reader.deserialize::<CsvTransaction>();
        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.transaction_type, CsvTransactionType::Deposit);
        assert_eq!(tx.client, 1);
        assert_eq!(tx.tx, 1001);
        assert_eq!(tx.amount, Some(dec!(42.5)));
    }

    #[test]
    fn test_deserialize_withdrawal() {
        let data = "type,client,tx,amount\n\
                    withdrawal,2,1002,10.0\n";
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data.as_bytes());
        let mut iter = reader.deserialize::<CsvTransaction>();
        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.transaction_type, CsvTransactionType::Withdrawal);
        assert_eq!(tx.client, 2);
        assert_eq!(tx.tx, 1002);
        assert_eq!(tx.amount, Some(dec!(10.0)));
    }

    #[test]
    fn test_deserialize_dispute() {
        let data = "type,client,tx,amount\n\
                    dispute,3,1003,\n";
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data.as_bytes());
        let mut iter = reader.deserialize::<CsvTransaction>();
        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.transaction_type, CsvTransactionType::Dispute);
        assert_eq!(tx.client, 3);
        assert_eq!(tx.tx, 1003);
        assert_eq!(tx.amount, None);
    }
}

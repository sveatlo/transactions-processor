mod account;
mod transaction;

use std::collections::HashMap;

use thiserror::Error;
use tracing::warn;
pub use transaction::Transaction;
pub use transaction::TransactionType;

use crate::payment_engine::account::AccountStatus;

#[derive(Debug, Clone, Default)]
pub struct PaymentEngine {
    clients: HashMap<u16, AccountStatus>,
    transactions: HashMap<u32, Transaction>,
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), PaymentEngineError> {
        let client = self
            .clients
            .entry(transaction.client)
            .or_insert_with(|| AccountStatus::new(transaction.client));

        match transaction.r#type {
            TransactionType::Deposit { amount } => {
                if amount < 0.0 {
                    return Err(PaymentEngineError::InvalidAmount(
                        amount,
                        "deposit amount cannot be negative".to_string(),
                    ));
                }

                client.deposit(amount)?;
                self.transactions.insert(transaction.id, transaction);
            }
            TransactionType::Withdrawal { amount } => {
                if amount < 0.0 {
                    return Err(PaymentEngineError::InvalidAmount(
                        amount,
                        "withdrawal amount cannot be negative".to_string(),
                    ));
                }

                let res = client.withdraw(amount);

                if !matches!(res, Err(PaymentEngineError::InsufficientFunds)) {
                    res?;
                } else {
                    warn!(
                        transaction_id = transaction.id,
                        "insufficient funds for withdrawal"
                    );
                    return Ok(());
                }

                self.transactions.insert(transaction.id, transaction);
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                let Some(original_transaction) = self.transactions.get_mut(&transaction.id) else {
                    // Probably an error on our partner's side
                    warn!(transaction_id = transaction.id, "transaction not found");
                    return Ok(());
                };

                if original_transaction.client != transaction.client {
                    return Err(PaymentEngineError::DisputeForDifferentClient);
                }

                let amount = match original_transaction.r#type {
                    TransactionType::Deposit { amount } => amount,
                    TransactionType::Withdrawal { amount } => -amount,
                    _ => {
                        return Err(PaymentEngineError::InvalidTransactionType(
                            "dispute can only be applied to deposit or withdrawal".to_string(),
                        ));
                    }
                };

                match transaction.r#type {
                    TransactionType::Dispute => {
                        if original_transaction.is_disputed {
                            return Err(PaymentEngineError::TransactionAlreadyDisputed(
                                transaction.id,
                            ));
                        }

                        original_transaction.is_disputed = true;
                        client.hold_funds(amount)?;
                    }
                    TransactionType::Resolve => {
                        if !original_transaction.is_disputed {
                            warn!(
                                transaction_id = transaction.id,
                                "transaction was not disputed"
                            );
                            return Ok(());
                        }

                        original_transaction.is_disputed = false;
                        client.release_funds(amount);
                    }
                    TransactionType::Chargeback => {
                        if !original_transaction.is_disputed {
                            warn!(
                                transaction_id = transaction.id,
                                "transaction was not disputed"
                            );
                            return Ok(());
                        }

                        original_transaction.is_disputed = false;
                        client.chargeback(amount);
                    }
                    TransactionType::Deposit { .. } | TransactionType::Withdrawal { .. } => {
                        unreachable!()
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum PaymentEngineError {
    #[error("insufficient funds for withdrawal")]
    InsufficientFunds,
    #[error("account is locked: {0}")]
    AccountLocked(u16),
    #[error("invalid transaction type: {0}")]
    InvalidTransactionType(String),
    #[error("invalid transaction amount: {0} - {1}")]
    InvalidAmount(f64, String),
    #[error("transaction (id={0}) is already disputed")]
    TransactionAlreadyDisputed(u32),
    #[error("dispute operations can only be applied to the same client account")]
    DisputeForDifferentClient,
}

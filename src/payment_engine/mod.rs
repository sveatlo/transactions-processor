mod account;
mod transaction;

use std::collections::HashMap;

use rust_decimal::Decimal;
use thiserror::Error;
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
                if amount < Decimal::ZERO {
                    return Err(PaymentEngineError::InvalidAmount(
                        amount,
                        "deposit amount cannot be negative".to_string(),
                    ));
                }

                client.deposit(amount)?;
                self.transactions.insert(transaction.id, transaction);
            }
            TransactionType::Withdrawal { amount } => {
                if amount < Decimal::ZERO {
                    return Err(PaymentEngineError::InvalidAmount(
                        amount,
                        "withdrawal amount cannot be negative".to_string(),
                    ));
                }

                client.withdraw(amount)?;
                self.transactions.insert(transaction.id, transaction);
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                let Some(original_transaction) = self.transactions.get_mut(&transaction.id) else {
                    return Err(PaymentEngineError::TransactionNotFound(transaction.id));
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
                            return Err(PaymentEngineError::NotDisputed(transaction.id));
                        }

                        original_transaction.is_disputed = false;
                        client.release_funds(amount);
                    }
                    TransactionType::Chargeback => {
                        if !original_transaction.is_disputed {
                            return Err(PaymentEngineError::NotDisputed(transaction.id));
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

    pub fn get_accounts_statuses(&self) -> Vec<AccountStatus> {
        self.clients.values().cloned().collect()
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
    InvalidAmount(Decimal, String),
    #[error("transaction (id={0}) not found")]
    TransactionNotFound(u32),
    #[error("transaction (id={0}) is already disputed")]
    TransactionAlreadyDisputed(u32),
    #[error("transaction (id={0}) was not disputed")]
    NotDisputed(u32),
    #[error("dispute operations can only be applied to the same client account")]
    DisputeForDifferentClient,
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;
    use crate::payment_engine::TransactionType;

    #[test]
    fn test_deposit_and_withdrawal() {
        let mut engine = PaymentEngine::new();
        let deposit = Transaction::new(
            1,
            1,
            TransactionType::Deposit {
                amount: dec!(100.0),
            },
        );
        let withdrawal = Transaction::new(1, 2, TransactionType::Withdrawal { amount: dec!(40.0) });

        engine.process_transaction(deposit).unwrap();
        engine.process_transaction(withdrawal).unwrap();

        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(60.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(60.0));
        assert!(!account.locked);
    }

    #[test]
    fn test_dispute_resolve_chargeback() {
        let mut engine = PaymentEngine::new();
        let deposit = Transaction::new(
            1,
            1,
            TransactionType::Deposit {
                amount: dec!(100.0),
            },
        );
        let dispute = Transaction::new(1, 1, TransactionType::Dispute);
        let resolve = Transaction::new(1, 1, TransactionType::Resolve);
        let chargeback = Transaction::new(1, 1, TransactionType::Chargeback);

        engine.process_transaction(deposit).unwrap();

        // Dispute
        engine.process_transaction(dispute.clone()).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(100.0));

        // Resolve
        engine.process_transaction(resolve).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));

        // Dispute again and chargeback
        engine.process_transaction(dispute).unwrap();
        engine.process_transaction(chargeback).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(0.0));
        assert!(account.locked);
    }

    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut engine = PaymentEngine::new();
        let deposit = Transaction::new(1, 1, TransactionType::Deposit { amount: dec!(50.0) });
        let withdrawal = Transaction::new(
            1,
            2,
            TransactionType::Withdrawal {
                amount: dec!(100.0),
            },
        );

        engine.process_transaction(deposit).unwrap();
        let result = engine.process_transaction(withdrawal);
        assert!(matches!(result, Err(PaymentEngineError::InsufficientFunds)));
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(50.0));
        assert_eq!(account.total, dec!(50.0));
    }

    #[test]
    fn test_multiple_disputes_and_resolves() {
        let mut engine = PaymentEngine::new();
        let deposit1 = Transaction::new(
            1,
            1,
            TransactionType::Deposit {
                amount: dec!(100.0),
            },
        );
        let deposit2 = Transaction::new(1, 2, TransactionType::Deposit { amount: dec!(50.0) });
        let withdrawal = Transaction::new(1, 3, TransactionType::Withdrawal { amount: dec!(30.0) });

        engine.process_transaction(deposit1).unwrap();
        engine.process_transaction(deposit2).unwrap();
        engine.process_transaction(withdrawal).unwrap();

        // Dispute deposit1
        let dispute1 = Transaction::new(1, 1, TransactionType::Dispute);
        engine.process_transaction(dispute1).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(20.0)); // 100+50-30-100(disputed)
        assert_eq!(account.held, dec!(100.0));
        assert_eq!(account.total, account.held + account.available);

        // Dispute deposit2
        let dispute2 = Transaction::new(1, 2, TransactionType::Dispute);
        engine.process_transaction(dispute2).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(-30.0)); // 20-50(disputed)
        assert_eq!(account.held, dec!(150.0));
        assert_eq!(account.total, account.held + account.available);

        // Resolve deposit1
        let resolve1 = Transaction::new(1, 1, TransactionType::Resolve);
        engine.process_transaction(resolve1).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(70.0)); // -30+100(resolved)
        assert_eq!(account.held, dec!(50.0));
        assert_eq!(account.total, account.held + account.available);

        // Chargeback deposit2
        let chargeback2 = Transaction::new(1, 2, TransactionType::Chargeback);
        engine.process_transaction(chargeback2).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
            .unwrap();
        assert_eq!(account.available, dec!(70.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(70.0));
        assert!(account.locked);
    }

    #[test]
    fn test_dispute_withdrawal_and_chargeback() {
        let mut engine = PaymentEngine::new();
        let deposit = Transaction::new(
            2,
            1,
            TransactionType::Deposit {
                amount: dec!(200.0),
            },
        );
        let withdrawal = Transaction::new(2, 2, TransactionType::Withdrawal { amount: dec!(50.0) });

        engine.process_transaction(deposit).unwrap();
        engine.process_transaction(withdrawal).unwrap();

        // Dispute withdrawal
        let dispute_withdrawal = Transaction::new(2, 2, TransactionType::Dispute);
        engine.process_transaction(dispute_withdrawal).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 2)
            .unwrap();
        assert_eq!(account.available, dec!(200.0)); // 200-50+50(held)
        assert_eq!(account.held, dec!(-50.0)); // held is negative for withdrawal dispute
        assert_eq!(account.total, account.held + account.available);

        // Chargeback withdrawal
        let chargeback_withdrawal = Transaction::new(2, 2, TransactionType::Chargeback);
        engine.process_transaction(chargeback_withdrawal).unwrap();
        let account = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 2)
            .unwrap();
        assert_eq!(account.available, dec!(200.0));
        assert_eq!(account.held, dec!(0.0));
        assert_eq!(account.total, dec!(200.0));
        assert!(account.locked);
    }

    #[test]
    fn test_negative_deposit_and_withdrawal() {
        let mut engine = PaymentEngine::new();
        let deposit = Transaction::new(
            1,
            1,
            TransactionType::Deposit {
                amount: dec!(-10.0),
            },
        );
        let withdrawal = Transaction::new(
            1,
            2,
            TransactionType::Withdrawal {
                amount: dec!(-20.0),
            },
        );

        let result_deposit = engine.process_transaction(deposit);
        assert!(matches!(
            result_deposit,
            Err(PaymentEngineError::InvalidAmount(_, _))
        ));

        let result_withdrawal = engine.process_transaction(withdrawal);
        assert!(matches!(
            result_withdrawal,
            Err(PaymentEngineError::InvalidAmount(_, _))
        ));
    }

    #[test]
    fn test_dispute_nonexistent_transaction() {
        let mut engine = PaymentEngine::new();
        let dispute = Transaction::new(1, 99, TransactionType::Dispute);

        let result = engine.process_transaction(dispute);
        assert!(matches!(
            result,
            Err(PaymentEngineError::TransactionNotFound(99))
        ));
        let Some(account) = engine
            .get_accounts_statuses()
            .into_iter()
            .find(|a| a.client == 1)
        else {
            panic!("account should exist")
        };

        assert_eq!(account.available, dec!(0.0));
    }

    #[test]
    fn test_dispute_for_different_client() {
        let mut engine = PaymentEngine::new();
        let deposit = Transaction::new(
            1,
            1,
            TransactionType::Deposit {
                amount: dec!(100.0),
            },
        );
        engine.process_transaction(deposit).unwrap();

        let dispute = Transaction::new(2, 1, TransactionType::Dispute);
        let result = engine.process_transaction(dispute);
        assert!(matches!(
            result,
            Err(PaymentEngineError::DisputeForDifferentClient)
        ));
    }
}

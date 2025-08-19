use rust_decimal::Decimal;
use serde::Serialize;

use crate::payment_engine::PaymentEngineError;

#[derive(Serialize, Debug, Clone)]
pub struct AccountStatus {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl AccountStatus {
    pub fn new(client_id: u16) -> Self {
        Self {
            client: client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            total: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: Decimal) -> Result<(), PaymentEngineError> {
        if self.locked {
            return Err(PaymentEngineError::AccountLocked(self.client));
        }

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal) -> Result<(), PaymentEngineError> {
        if self.locked {
            return Err(PaymentEngineError::AccountLocked(self.client));
        }

        if self.available < amount {
            return Err(PaymentEngineError::InsufficientFunds);
        }

        self.available -= amount;
        self.total -= amount;

        Ok(())
    }

    pub fn hold_funds(&mut self, amount: Decimal) -> Result<(), PaymentEngineError> {
        self.available -= amount;
        self.held += amount;

        Ok(())
    }

    pub fn release_funds(&mut self, amount: Decimal) {
        self.held -= amount;
        self.available += amount;
    }

    pub fn chargeback(&mut self, amount: Decimal) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }
}

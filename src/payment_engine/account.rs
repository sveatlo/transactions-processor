use serde::Serialize;

use crate::payment_engine::PaymentEngineError;

#[derive(Serialize, Debug, Clone)]
pub struct AccountStatus {
    pub client: u16,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

impl AccountStatus {
    pub fn new(client_id: u16) -> Self {
        Self {
            client: client_id,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: f64) -> Result<(), PaymentEngineError> {
        if self.locked {
            return Err(PaymentEngineError::AccountLocked(self.client));
        }

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    pub fn withdraw(&mut self, amount: f64) -> Result<(), PaymentEngineError> {
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

    pub fn hold_funds(&mut self, amount: f64) -> Result<(), PaymentEngineError> {
        self.available -= amount;
        self.held += amount;

        Ok(())
    }

    pub fn release_funds(&mut self, amount: f64) {
        self.held -= amount;
        self.available += amount;
    }

    pub fn chargeback(&mut self, amount: f64) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }
}

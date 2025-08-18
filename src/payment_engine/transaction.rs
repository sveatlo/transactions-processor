#[derive(Debug, Clone)]
pub struct Transaction {
    pub client: u16,
    pub transaction_id: u32,
    pub(super) is_disputed: bool,
    pub r#type: TransactionType,
}

#[derive(Debug, Clone)]
pub enum TransactionType {
    Deposit { amount: f64 },
    Withdrawal { amount: f64 },
    Dispute,
    Resolve,
    Chargeback,
}

impl Transaction {
    pub fn new(client: u16, transaction_id: u32, r#type: TransactionType) -> Self {
        Self {
            client,
            transaction_id,
            is_disputed: false,
            r#type,
        }
    }
}

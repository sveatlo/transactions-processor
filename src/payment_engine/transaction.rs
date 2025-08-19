use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub client: u16,
    pub id: u32,
    pub(super) is_disputed: bool,
    pub r#type: TransactionType,
}

#[derive(Debug, Clone)]
pub enum TransactionType {
    Deposit { amount: Decimal },
    Withdrawal { amount: Decimal },
    Dispute,
    Resolve,
    Chargeback,
}

impl Transaction {
    pub fn new(client: u16, transaction_id: u32, r#type: TransactionType) -> Self {
        Self {
            client,
            id: transaction_id,
            is_disputed: false,
            r#type,
        }
    }
}

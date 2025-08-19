# Payments Engine

A simple toy payments engine that processes financial transactions from CSV files, handling deposits, withdrawals, disputes, resolutions, and chargebacks.

## Features

- **Transaction Processing**: Handles deposits, withdrawals, disputes, resolves, and chargebacks
- **Account Management**: Tracks available, held, and total funds for each client
- **Dispute Handling**: Supports the full dispute lifecycle from dispute to resolution or chargeback
- **Safety**: Prevents insufficient fund withdrawals and locks accounts after chargebacks
- **Precision**: Uses `f64` for financial calculations
- **Streaming**: Processes CSV files line by line for memory efficiency with large datasets
- **Error Handling**: Robust error handling with detailed error types

## Usage

```bash
cargo run -- transactions.csv > accounts.csv
```

## Input Format

The input CSV should have columns: `type`, `client`, `tx`, and `amount`.

Example:

```csv
type,client,tx,amount
deposit,1,1,1.0
deposit,2,2,2.0
withdrawal,1,3,1.5
dispute,1,1,
resolve,1,1,
```

## Output Format

The output CSV contains: `client`, `available`, `held`, `total`, and `locked`.

Example:

```csv
client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
```

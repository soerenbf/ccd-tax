use std::{collections::HashSet, hash::Hash};

use chrono::{DateTime, Utc};
use clap::Parser;
use concordium_rust_sdk::{
    base::hashes::TransactionHash, common::types::Amount, id::types::AccountAddress,
};
use serde::{Deserialize, Deserializer};

const URL: &str = "https://wallet-proxy.mainnet.concordium.software";

#[derive(Parser, Debug)]
#[clap(name = "AccountAddressList")]
struct Args {
    #[clap(short, long = "account")]
    accounts: Vec<AccountAddress>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Details {
    Transfer {
        #[serde(rename = "transferSource")]
        from: AccountAddress,
        #[serde(rename = "transferDestination")]
        to: AccountAddress,
        #[serde(rename = "transferAmount")]
        amount: Amount,
    },
    PaydayAccountReward {},
    ConfigureDelegation {},
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    #[serde(rename = "transactionHash")]
    hash: TransactionHash,
    #[serde(deserialize_with = "deserialize_block_time")]
    block_time: DateTime<Utc>,
    details: Details,
    cost: Option<Amount>,
}

impl Hash for Transaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Transaction {}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TransactionsResponse {
    count: u16,
    limit: u16,
    transactions: Vec<Transaction>,
}

fn deserialize_block_time<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = f64::deserialize(deserializer)?;
    let time: DateTime<Utc> = chrono::DateTime::from_timestamp_millis((timestamp * 1000.0) as i64)
        .expect("Can convert timestamp");
    Ok(time)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut transactions = HashSet::new();

    for account in &args.accounts {
        let res: TransactionsResponse =
            reqwest::get(format!("{URL}/v1/accTransactions/{}?limit=1000", account))
                .await?
                .json()
                .await?;

        transactions.extend(res.transactions);
    }

    println!("success");

    Ok(())
}

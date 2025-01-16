use std::collections::BTreeSet;

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
    #[clap(short = 'a', long = "account")]
    accounts: Vec<AccountAddress>,
    #[clap(short = 'l', long = "api-limit", default_value = "100")]
    api_limit: u16,
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    #[serde(rename = "transactionHash")]
    hash: Option<TransactionHash>,
    #[serde(deserialize_with = "deserialize_block_time")]
    block_time: DateTime<Utc>,
    details: Details,
    cost: Option<Amount>,
    id: u64,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Transaction {}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.block_time.cmp(&other.block_time))
    }
}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.block_time.cmp(&other.block_time)
    }
}

#[derive(Deserialize, Debug, Clone)]
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

async fn request_transactions(
    account: &AccountAddress,
    limit: u16,
    from: Option<u64>,
) -> anyhow::Result<(TransactionsResponse, bool)> {
    let mut url = format!("{URL}/v1/accTransactions/{account}?limit={limit}");
    if let Some(from) = from {
        url.push_str(&format!("&from={from}"));
    }
    let res: TransactionsResponse = reqwest::get(url).await?.json().await?;
    let has_more = res.count == res.limit;

    Ok((res, has_more))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut transactions = BTreeSet::new();

    for account in &args.accounts {
        let mut from = None;
        loop {
            let (res, has_more) = request_transactions(account, args.api_limit, from).await?;
            transactions.extend(res.transactions.clone());

            if !has_more { break; }
            let Some(tx) = res.transactions.last() else { break; };

            from = Some(tx.id);
        }
    }

    println!("success {}", transactions.len());

    Ok(())
}

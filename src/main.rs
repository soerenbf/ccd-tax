use std::collections::BTreeSet;

use anyhow::Context;
use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use concordium_rust_sdk::{
    base::hashes::TransactionHash,
    common::types::Amount,
    id::types::AccountAddress,
};
use serde::{Deserialize, Deserializer, Serialize};

const URL: &str = "https://wallet-proxy.mainnet.concordium.software";

#[derive(Parser, Debug)]
#[clap(name = "AccountAddressList")]
struct Args {
    /// The accounts to include in the result. These are also used to exclude transactions where
    /// both sender and receiver is in the list, as these are internal transfers with no relevance
    /// for tax purposes.
    #[clap(short = 'a', long = "account")]
    accounts: Vec<AccountAddress>,
    /// The amount of transactions to request per request made to the API.
    #[clap(short = 'l', long = "api-limit", default_value = "100")]
    api_limit: u16,
    /// The output format. Currently only "koinly" is supported
    #[clap(value_enum, default_value_t = Format::Koinly)]
    format: Format,
}

#[derive(Debug, Clone, ValueEnum)]
enum Format {
    Koinly,
}

#[derive(Debug, Serialize)]
enum KoinlyLabel {
    Fee,
    Mining,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct KoinlyRow {
    #[serde(rename = "Koinly Date")]
    date: String,
    amount: f64,
    currency: String,
    label: Option<KoinlyLabel>,
    tx_hash: Option<TransactionHash>,
}

impl KoinlyRow {
    fn new_ccd(
        date: String,
        amount: f64,
        label: Option<KoinlyLabel>,
        tx_hash: Option<TransactionHash>,
    ) -> Self {
        Self {
            date,
            amount,
            currency: "CCD".to_string(),
            label,
            tx_hash,
        }
    }
}

impl TryFrom<&Transaction> for Vec<KoinlyRow> {
    type Error = anyhow::Error;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
        let total = tx.total.context("no amount found")?;
        let amount = tx.subtotal.unwrap_or(total) as f64 / 1_000_000.0;
        let label = match tx.details {
            Details::PaydayAccountReward {} => Some(KoinlyLabel::Mining),
            _ => None,
        };

        let value = KoinlyRow::new_ccd(
            tx.block_time
                .naive_utc()
                .format("%Y-%m-%d %H:%M UTC")
                .to_string(),
            amount,
            label,
            tx.hash,
        );

        let Some(cost) = tx.cost else {
            return Ok(vec![value]);
        };

        let fee = KoinlyRow::new_ccd(
            tx.block_time
                .naive_utc()
                .format("%Y-%m-%d %H:%M UTC")
                .to_string(),
            -(cost.micro_ccd as f64 / 1_000_000.0),
            Some(KoinlyLabel::Fee),
            tx.hash,
        );

        if Amount::from_micro_ccd(total.unsigned_abs()) == cost {
            // We're not transferring any funds, only paying a fee.
            return Ok(vec![fee]);
        }
        return Ok(vec![value, fee]);
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Details {
    // The addresses are used to figure out if the transfer is internal or not.
    Transfer {
        #[serde(rename = "transferSource")]
        from: AccountAddress,
        #[serde(rename = "transferDestination")]
        to: AccountAddress,
    },
    // The details of other transactions are not of interest for this specific use-case.
    PaydayAccountReward {},
    // Catch-all makes sure don't crash on transactions where the details are not of interest.
    #[serde(untagged)]
    Other {},
}

fn deserialize_micro_ccd<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    opt.map(|s| s.parse::<i64>().map_err(serde::de::Error::custom))
        .transpose()
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Transaction {
    #[serde(rename = "transactionHash")]
    hash: Option<TransactionHash>, // Not available for reward types
    // block_hash: BlockHash, // Can be used as a reference when looking up rewards for the receiver
    #[serde(deserialize_with = "deserialize_block_time")]
    block_time: DateTime<Utc>,
    details: Details,
    cost: Option<Amount>, // Not available for reward types
    #[serde(default, deserialize_with = "deserialize_micro_ccd")]
    subtotal: Option<i64>, // Contains signed amount in micro CCD excluding the `cost`
    #[serde(deserialize_with = "deserialize_micro_ccd")]
    total: Option<i64>, // Contains signed amount in micro CCD
    id: u64,
}

// Avoid duplicates by using the ID from the DB
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

            if !has_more {
                break;
            }
            let Some(tx) = res.transactions.last() else {
                break;
            };

            from = Some(tx.id);
        }
    }

    println!("pre filter {}", &transactions.len());
    transactions.retain(|tx| !matches!(tx.details, Details::Transfer { from, to } if args.accounts.contains(&from) && args.accounts.contains(&to)));
    println!("success {}", &transactions.len());

    let formatted: Vec<KoinlyRow> = transactions
        .iter()
        .filter_map(|tx| Vec::<KoinlyRow>::try_from(tx).ok())
        .flatten()
        .collect();

    for row in formatted.iter() {
        println!("{:?}", row);
    }

    Ok(())
}

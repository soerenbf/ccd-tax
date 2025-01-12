use clap::Parser;
use concordium_rust_sdk::{common::types::Amount, id::types::AccountAddress};
use serde::{Deserialize, Deserializer};
use chrono::{DateTime, Utc};

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
    #[serde(deserialize_with = "deserialize_block_time")]
    block_time: DateTime<Utc>,
    details: Details,
    cost: Option<Amount>,
}

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
    let time: DateTime<Utc> = chrono::DateTime::from_timestamp_millis((timestamp * 1000.0) as i64).expect("Can convert timestamp");
    Ok(time)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    for account in &args.accounts {
        let res: TransactionsResponse =
            reqwest::get(format!("{URL}/v1/accTransactions/{}?limit=1000", account))
                .await?
                .json()
                .await?;

        println!("count {}", res.count);
    }

    println!("success");

    Ok(())
}

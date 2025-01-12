use clap::Parser;
use concordium_rust_sdk::id::types::AccountAddress;

#[derive(Parser, Debug)]
#[clap(name = "AccountAddressList")]
struct Args {
    #[clap(short, long = "account")]
    accounts: Vec<AccountAddress>,
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}

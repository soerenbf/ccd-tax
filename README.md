# CCD Tax calculation script

```sh
Usage: ccd-tax [OPTIONS] [FORMAT]

Arguments:
[FORMAT]  The output format. Currently only "koinly" is supported [default: koinly] [possible values: koinly]

Options:
-a, --account <ACCOUNTS>     The accounts to include in the result. These are also used to exclude transactions where both sender and receiver is in the list, as these are internal transfers with no relevance for tax purposes
-l, --api-limit <API_LIMIT>  The amount of transactions to request per request made to the API [default: 100]
-o, --output <OUTPUT>        The output file path
-h, --help                   Print help
```

## Example 

```sh
# run from binary:
ccd-tax -a "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd" -a "3ybJ66spZ2xdWF3avgxQb2meouYa7mpvMWNPmUnczU8FoF8cGB" -o output.csv

# or alternatively to run with cargo:
cargo run --release -- -a "4UC8o4m8AgTxt5VBFMdLwMCwwJQVJwjesNzW7RPXkACynrULmd" -a "3ybJ66spZ2xdWF3avgxQb2meouYa7mpvMWNPmUnczU8FoF8cGB" -o output.csv
```

The example above collects all transactions for the 2 accounts specified and puts it into `./output.csv`. Can be imported into [Koinly](https://koinly.io/).

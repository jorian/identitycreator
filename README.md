# Identity Creator

This app lets you create Verus identities in an automated fashion.

## How to use

This is an asynchronous app, so a runtime like `tokio` is required.

Add

```toml
tokio = {version = "1", features = ["full"]}
```

to your Cargo.toml.

Then, build your identity:

```rs
use std::str::FromStr;

use identitycreator::*;
use vrsc::Address;

#[tokio::main]
async fn main() {
    // It is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
    let identity_result = Identity::builder()
        .testnet(true) // (optional) default: false
        .on_currency_name("geckotest") // (optional) when given, the name of the currency is used as a parent to issue sub-ids on
        .name("aaaaad") // (required) the name of the identity
        .referral("aaaaab.geckotest@") // (optional) referral
        .add_address(Address::from_str("RP1sexQNvjGPohJkK9JnuPDH7V7NboycGj").unwrap()) // (required) at least 1 primary address
        .add_private_address(
            "zs1pf0pjumxr6k5zdwupl8tnl58gqrpklznxhypjlzp3reaqpxdh0ce7qj2u7qfp8z8mc9pc39epgm",
        ) // (optional)
        .minimum_signatures(1) // (optional)
        .create()
        .await;

    match identity_result {
        Ok(identity) => {
            info!(
                "identity `{}` has been created! (txid: {})",
                identity.name_commitment.namereservation.name,
                identity.registration_txid
            )
        }
        Err(e) => {
            error!("something went wrong: {:?}", e)
        }
    }
}
```

## Dependencies

This library depends on the `vrsc-rpc` library, which is a RPC wrapper for the Verus daemon. It is required to have a native Verus instance running.

## Todo

- make use of zmq to wait for new block, instead of polling
- add contentmap
- ~~make async~~

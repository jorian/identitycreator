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
use vrsc_rpc::jsonrpc::serde_json::json;

#[tokio::main]
async fn main() {
    // It is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
    if let Ok(identity_builder) = Identity::builder()
        .testnet(true)
        .on_currency_name("geckotest")
        .name("aaaaad")
        // .referral("aaaaab.geckotest@")
        .add_address(Address::from_str("RP1sexQNvjGPohJkK9JnuPDH7V7NboycGj").unwrap())
        .add_private_address(
            "zs1pf0pjumxr6k5zdwupl8tnl58gqrpklznxhypjlzp3reaqpxdh0ce7qj2u7qfp8z8mc9pc39epgm",
        )
        .minimum_signatures(1)
        .with_content_map(json!({ "deadbeef": "deadbeef"}))
        .validate()
    {
        let identity_result = identity_builder.create_identity().await;

        match identity_result {
            Ok(identity) => {
                info!(
                    "identity `{}` has been created! (txid: {})",
                    identity.name_commitment.namereservation.name, identity.registration_txid
                )
            }
            Err(e) => {
                error!("something went wrong: {:?}", e)
            }
        }
    }
}
```

## Dependencies

This library depends on the `vrsc-rpc` library, which is a RPC wrapper for the Verus daemon. It is required to have a native Verus instance running.

## Todo

- make use of zmq to wait for new block, instead of polling
- incorporate the `updateidentity` call
- ~~add contentmap~~
- ~~make async~~

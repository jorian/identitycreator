use std::str::FromStr;

use identitycreator::*;
use tracing::*;
use tracing_subscriber::filter::EnvFilter;
use vrsc::Address;
use vrsc_rpc::jsonrpc::serde_json::json;

#[tokio::main]
async fn main() {
    setup_logging();

    info!("creating identity");

    // It is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
    if let Ok(identity_builder) = Identity::builder()
        .testnet(true)
        .on_currency_name("geckotest")
        .name("aaaaah")
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

fn setup_logging() {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install().unwrap();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "vrsc_rpc=info,identitycreator=debug")
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

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
        // .on_currency_name("geckotest")
        .name("geckotest")
        // .referral("aaaaab.geckotest@")
        .add_address(Address::from_str("RTkW5eTcmcYMdibeby4bEoDJpCEe6EepY1").unwrap())
        .add_private_address(
            "zs1nd372rzvccg9njpdafvxcq8zcqlfmcxyfn3rdmgfe879x77qt23h9f6t5f2q7zecz5jws0ame79",
        )
        .minimum_signatures(1)
        // .with_content_map(json!({ "deadbeef": "deadbeef"}))
        .validate()
    {
        debug!("{:#?}", identity_builder);

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

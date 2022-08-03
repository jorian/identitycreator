use std::str::FromStr;

use tracing::*;
use tracing_subscriber::filter::EnvFilter;

use identitycreator::*;
use vrsc::Address;

fn main() {
    setup_logging();

    info!("creating identity");

    // It is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
    let identity_result = Identity::builder()
        .testnet(true)
        .on_currency_name("geckotest")
        .name("aaaaac")
        .referral("aaaaab.geckotest@")
        .add_address(Address::from_str("RP1sexQNvjGPohJkK9JnuPDH7V7NboycGj").unwrap())
        .minimum_signatures(1)
        .create();

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

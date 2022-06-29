use std::{error::Error, str::FromStr, thread, time::Duration};

use vrsc::Address;
use vrsc_rpc::{bitcoin::Txid, json::identity::NameCommitment, Client, RpcApi};

use tracing::*;
use tracing_subscriber::filter::EnvFilter;

#[macro_use]
extern crate derive_more;

// TODO create an in-between build step to catch mistakes and impossibilities
// - double addresses

fn main() {
    setup_logging();

    info!("creating identity");

    // it is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
    match Identity::builder()
        .name("joriankilo")
        .referral("jorian@")
        .add_address(Address::from_str("RLGn1rQMUKcy5Yh2xNts7U9bd9SvF7k6uE").unwrap())
        .add_private_address(
            "zs1e0s04c8swwrvzsa06cpa8suv70n0uftdnfy34je5fx2vz54ny4wttvl43ezy3kqmau6zc93kxr6",
        )
        .minimum_signatures(1)
        .create()
    {
        Ok(identity) => {
            info!("identity created:\n\n{:?}", identity)
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

#[derive(Debug)]
pub struct Identity {
    registration_txid: Txid,
    name: String,
    name_commitment: NameCommitment,
}

impl Identity {
    pub fn builder() -> IdentityBuilder {
        IdentityBuilder {
            pbaas: None,
            name: None,
            referral: None,
            minimum_signatures: None,
            addresses: None,
            private_address: None,
        }
    }
}

pub struct IdentityBuilder {
    pbaas: Option<String>,
    name: Option<String>,
    referral: Option<String>,
    // defaults to 1
    minimum_signatures: Option<u8>,
    addresses: Option<Vec<Address>>,
    private_address: Option<String>,
}

impl IdentityBuilder {
    pub fn on_pbaas_chain(&mut self, s: &str) -> &mut Self {
        self.pbaas = Some(String::from(s));

        self
    }

    pub fn name(&mut self, s: &str) -> &mut Self {
        self.name = Some(String::from(s));

        self
    }

    pub fn referral(&mut self, s: &str) -> &mut Self {
        self.referral = Some(String::from(s));

        self
    }

    pub fn minimum_signatures(&mut self, s: u8) -> &mut Self {
        self.minimum_signatures = Some(s);

        self
    }

    pub fn add_address(&mut self, address: Address) -> &mut Self {
        match self.addresses.as_mut() {
            Some(vec) => {
                vec.push(address);
            }
            None => {
                self.addresses = Some(vec![address]);
            }
        }

        self
    }

    pub fn add_private_address(&mut self, s: &str) -> &mut Self {
        self.private_address = Some(String::from(s));

        self
    }

    pub fn create(&mut self) -> Result<Identity, IdentityError> {
        // TODO if minimum_signatures > amount of addresses, error

        let client = match &self.pbaas {
            Some(chain) => Client::chain(&chain, vrsc_rpc::Auth::ConfigFile),
            None => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile),
        }?;
        let name_commitment = client.registernamecommitment(
            self.name.clone().unwrap().as_ref(),
            self.addresses.clone().unwrap().first().unwrap(),
            self.referral.clone(),
        )?;

        let txid = name_commitment.txid;
        debug!("{}", &txid);

        loop {
            thread::sleep(Duration::from_secs(3));
            // TODO implement get_raw_transaction fix: https://github.com/VerusCoin/VerusCoin/issues/432
            match client.get_transaction(&txid, Some(false)) {
                Ok(tx) => {
                    if tx.confirmations > 0 {
                        // the identity can now by registered.
                        let registration_txid = client.registeridentity(
                            &name_commitment,
                            self.addresses.clone().unwrap(),
                            self.minimum_signatures,
                            self.private_address.clone(),
                        )?;

                        return Ok(Identity {
                            registration_txid,
                            name: String::from(&name_commitment.namereservation.name),
                            name_commitment,
                        });
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

#[derive(Debug, Display)]
#[display(fmt = "{}", kind)]
pub struct IdentityError {
    pub kind: ErrorKind,
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

#[derive(Debug, Display)]
pub enum ErrorKind {
    #[display(fmt = "Something went wrong during the komodod RPC.")]
    ApiError(vrsc_rpc::Error),
    Other(String),
    // todo nonexhaustive to not have a breaking change when adding an error type
}

impl Error for IdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|boxed| boxed.as_ref() as &(dyn Error + 'static))
    }
}

impl From<vrsc_rpc::Error> for IdentityError {
    fn from(e: vrsc_rpc::Error) -> Self {
        ErrorKind::ApiError(e).into()
    }
}

impl From<ErrorKind> for IdentityError {
    fn from(kind: ErrorKind) -> Self {
        IdentityError { kind, source: None }
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

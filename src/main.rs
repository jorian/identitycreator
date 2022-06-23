use std::{error::Error, str::FromStr, thread, time::Duration};

use vrsc::Address;
use vrsc_rpc::{json::identity::NameCommitment, Client, RpcApi};

use tracing::*;
use tracing_subscriber::filter::EnvFilter;

#[macro_use]
extern crate derive_more;

// TODO create an in-between build step to catch mistakes and impossibilities
// - double addresses
// TODO add error handling

fn main() {
    setup_logging();

    info!("creating identity");
    let _identity = Identity::builder()
        // .on_pbaas_chain("veth")
        .name("jorianecho")
        .referral("jorian@")
        // it is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
        .add_address(Address::from_str("RLGn1rQMUKcy5Yh2xNts7U9bd9SvF7k6uE").unwrap())
        .add_address(Address::from_str("REwrXxaoo28BD5i8ycFEBK27jZXayGxyUj").unwrap())
        .create();
}

fn setup_logging() {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install().unwrap();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug")
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

pub struct Identity {}

impl Identity {
    pub fn builder() -> IdentityBuilder {
        IdentityBuilder {
            pbaas: None,
            name: None,
            referral: None,
            addresses: None,
        }
    }
}

pub struct IdentityBuilder {
    pbaas: Option<String>,
    name: Option<String>,
    referral: Option<String>,
    addresses: Option<Vec<Address>>,
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

    fn register_name_commitment(&mut self) -> Result<NameCommitment, IdentityError> {
        let client = match &self.pbaas {
            Some(chain) => Client::chain(&chain, vrsc_rpc::Auth::ConfigFile),
            None => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile),
        };

        if let Ok(client) = client {
            let commitment = client.registernamecommitment(
                self.name.clone().unwrap().as_ref(),
                self.addresses.clone().unwrap().first().unwrap(),
                self.referral.clone(),
            );

            match commitment {
                Ok(ncomm) => {
                    let txid = ncomm.txid;
                    dbg!(&txid);

                    loop {
                        thread::sleep(Duration::from_secs(3));
                        match client.get_transaction(&txid, Some(false)) {
                            Ok(tx) => {
                                if tx.confirmations > 0 {
                                    return Ok(ncomm);
                                }
                                debug!("txid.{} not confirmed", txid.to_string());
                            }
                            Err(e) => {
                                error!("{:?}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            };
        } else {
            info!("failed to start client");
        }

        Err(IdentityError {
            kind: ErrorKind::Other("unsuccessful".to_string()),
            source: None,
        })
    }

    fn register_identity(&self, namecommitment: NameCommitment) {
        let client = match &self.pbaas {
            Some(chain) => Client::chain(&chain, vrsc_rpc::Auth::ConfigFile),
            None => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile),
        };

        if let Ok(client) = client {
            let identity = client.registeridentity(namecommitment, self.addresses.clone().unwrap());
            debug!("{:?}", identity);

            info!("identity is created!")
        }
    }

    pub fn create(&mut self) -> Identity {
        let name_commitment = self.register_name_commitment();
        dbg!(&name_commitment);

        self.register_identity(name_commitment.unwrap());

        // TODO do the registeridentity call here.

        Identity {}
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

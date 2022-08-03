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

    // It is assumed that the first address that is pushed in the addresses array, will be the controlling address for the namecommitment.
    let _identity = Identity::builder()
        .testnet(true)
        .on_currency_name("geckotest")
        .name("aaaaaa")
        // .referral("jorian@")
        .add_address(Address::from_str("RYZJLCWYze9Md4kH1CyYGufxTinKLZxSwo").unwrap())
        .minimum_signatures(1)
        .create();
}

#[derive(Debug)]
pub struct Identity {
    //     registration_txid: Txid,
//     name: String,
//     name_commitment: NameCommitment,
}

impl Identity {
    pub fn builder() -> IdentityBuilder {
        IdentityBuilder {
            testnet: false,
            currency_name: None,
            name: None,
            referral: None,
            minimum_signatures: None,
            addresses: None,
            private_address: None,
        }
    }
}

pub struct IdentityBuilder {
    testnet: bool,
    currency_name: Option<String>,
    name: Option<String>,
    referral: Option<String>,
    // defaults to 1
    minimum_signatures: Option<u8>,
    addresses: Option<Vec<Address>>,
    private_address: Option<String>,
}

impl IdentityBuilder {
    pub fn testnet(&mut self, testnet: bool) -> &mut Self {
        self.testnet = testnet;

        self
    }

    // Currently there is no way to convert a currency name to a currencyidhex.
    pub fn on_currency_name<I: Into<String>>(&mut self, currency_name: I) -> &mut Self {
        self.currency_name = Some(currency_name.into());

        // self
        unimplemented!("PBaaS chains use currencyidhex which are not supported yet")
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

    pub fn create(&mut self) -> Identity {
        // TODO if minimum_signatures > amount of addresses, error

        if self.name.is_none() {
            panic!("No identity name was given");
        }

        if self.addresses.is_none() || self.addresses.as_ref().unwrap().is_empty() {
            panic!("no primary address given, need at least 1");
        }

        let name_commitment = self.register_name_commitment();
        dbg!(&name_commitment);

        let identity_response = self.register_identity(name_commitment.unwrap());
        Identity {}
    }

    fn register_name_commitment(&mut self) -> Result<NameCommitment, IdentityError> {
        let client = match self.testnet {
            false => Client::chain("VRSC", vrsc_rpc::Auth::ConfigFile),
            true => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile),
        };

        if let Ok(client) = client {
            let commitment = client.registernamecommitment(
                self.name.clone().unwrap().as_ref(),
                self.addresses.clone().unwrap().first().unwrap(),
                self.referral.clone(),
                self.currency_name.clone(),
            )?;

            // match commitment {
            //     Ok(ncomm) => {
            let txid = commitment.txid;
            dbg!(&txid);

            loop {
                thread::sleep(Duration::from_secs(3));
                match client.get_transaction(&txid, Some(false)) {
                    Ok(tx) => {
                        if tx.confirmations > 0 {
                            return Ok(commitment);
                        }
                        debug!("txid.{} not confirmed", txid.to_string());
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        break;
                    }
                }
            }
            //     }
            //     Err(e) => {
            //         error!("{:?}", e);
            //     }
            // };
        } else {
            info!("failed to start client");
        }

        Err(IdentityError {
            kind: ErrorKind::Other("unsuccessful".to_string()),
            source: None,
        })
    }

    fn register_identity(&self, namecommitment: NameCommitment) {
        let client = match self.testnet {
            false => Client::chain("VRSC", vrsc_rpc::Auth::ConfigFile),
            true => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile),
        };

        if let Ok(client) = client {
            let identity = client.registeridentity(
                namecommitment,
                self.addresses.clone().unwrap(),
                self.minimum_signatures,
                self.private_address.clone(),
                self.currency_name.clone(),
            );
            debug!("{:?}", identity);

            info!("identity is created!")
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
    #[display(fmt = "Something went wrong while sending a request to the komodod RPC.")]
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

impl From<ErrorKind> for IdentityError {
    fn from(kind: ErrorKind) -> Self {
        IdentityError { kind, source: None }
    }
}

impl From<vrsc_rpc::Error> for IdentityError {
    fn from(e: vrsc_rpc::Error) -> Self {
        ErrorKind::ApiError(e).into()
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

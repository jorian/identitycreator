#[macro_use]
extern crate derive_more;
use std::{error::Error, thread, time::Duration};
use tracing::*;

use vrsc::Address;
use vrsc_rpc::{bitcoin::Txid, json::identity::NameCommitment, Client, RpcApi};

#[derive(Debug)]
pub struct Identity {
    pub name_commitment: NameCommitment,
    pub registration_txid: Txid,
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
        if let (Some(min_sigs), Some(addresses)) =
            (self.minimum_signatures, self.addresses.as_ref())
        {
            if min_sigs > addresses.len() as u8 {
                return Err(IdentityError {
                    kind: ErrorKind::Other(String::from(
                        "Cannot have more minimum_signatures than there are primary addresses",
                    )),
                    source: None,
                });
            }
        }

        if self.name.is_none() {
            panic!("No identity name was given");
        }

        if self.addresses.is_none() || self.addresses.as_ref().unwrap().is_empty() {
            panic!("no primary address given, need at least 1");
        }

        let name_commitment = self.register_name_commitment()?;
        debug!("{:?}", &name_commitment);

        let identity_response = self.register_identity(&name_commitment)?;

        Ok(Identity {
            registration_txid: identity_response,
            name_commitment: name_commitment,
        })
    }

    fn register_name_commitment(&mut self) -> Result<NameCommitment, IdentityError> {
        let client = match self.testnet {
            false => Client::chain("VRSC", vrsc_rpc::Auth::ConfigFile, None),
            true => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile, None),
        }?;

        let commitment = client.registernamecommitment(
            self.name.clone().unwrap().as_ref(),
            self.addresses.clone().unwrap().first().unwrap(),
            self.referral.clone(),
            self.currency_name.clone(),
        )?;

        let txid = commitment.txid;
        debug!("{}", &txid);

        let mut retries = 0;
        loop {
            match client.get_transaction(&txid, Some(false)) {
                Ok(tx) => {
                    if tx.confirmations > 0 {
                        return Ok(commitment);
                    }
                    debug!("txid.{} not confirmed", txid.to_string());
                    thread::sleep(Duration::from_secs(3));
                }
                Err(e) => {
                    error!("{:?}", e);
                    if e.to_string().contains("non-wallet transaction") {
                        if retries > 20000 {
                            error!("waited too long for non-wallet transaction, abort");
                            return Err(e.into());
                        }
                        error!("transaction not yet in wallet, wait");
                        thread::sleep(Duration::from_millis(100));
                        retries += 1;
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }
    }

    fn register_identity(&self, namecommitment: &NameCommitment) -> Result<Txid, IdentityError> {
        let client = match self.testnet {
            false => Client::chain("VRSC", vrsc_rpc::Auth::ConfigFile, None),
            true => Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile, None),
        }?;

        let id_txid = client.registeridentity(
            &namecommitment,
            self.addresses.clone().unwrap(),
            self.minimum_signatures,
            self.private_address.clone(),
            self.currency_name.clone(),
        )?;
        debug!("{:?}", id_txid);

        info!(
            "identity `{}` is created!",
            &namecommitment.namereservation.name
        );

        Ok(id_txid)
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
    VrscRpcError(vrsc_rpc::Error),
    Other(String),
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
        ErrorKind::VrscRpcError(e).into()
    }
}

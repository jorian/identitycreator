#[macro_use]
extern crate derive_more;
use std::{error::Error, thread, time::Duration};
use tracing::*;

use vrsc::Address;
use vrsc_rpc::{
    bitcoin::Txid, json::identity::NameCommitment, jsonrpc::serde_json::Value, Client, RpcApi,
};

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
            content_map: None,
        }
    }
}

#[derive(Debug)]
pub struct IdentityBuilder {
    testnet: bool,
    currency_name: Option<String>,
    name: Option<String>,
    referral: Option<String>,
    // defaults to 1
    minimum_signatures: Option<u8>,
    addresses: Option<Vec<Address>>,
    private_address: Option<String>,
    content_map: Option<Value>,
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

    pub fn with_content_map(&mut self, cm: Value) -> &mut Self {
        self.content_map = Some(cm);

        self
    }

    pub fn validate(&mut self) -> Result<&mut Self, IdentityError> {
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
            return Err(ErrorKind::Other(String::from("No identity name was given")).into());
        }

        if self.addresses.is_none() || self.addresses.as_ref().unwrap().is_empty() {
            return Err(ErrorKind::Other(String::from(
                "no primary address given, need at least 1",
            ))
            .into());
        }

        // a content map has certain limitations:
        // std::vector<std::pair<uint160, uint256>>
        // It is an opaque blob of 256 bits and has no integer operations.
        // it's an unsigned 160 bit integer, represented as an array of bytes. 1 byte is 8 bits, so 160 / 8 = 20 bytes.
        // it's an unsigned 256 bit integer, represented as an array of bytes. 1 byte is 8 bits, so 256 / 8 = 32 bytes.
        // it's represented in hex, which is base 16.
        // so, the limitations of contentmap seem to be that
        // - it has to be hex
        // - the key must be 20 bytes long
        // - the value must be 32 bytes long.
        // any shorter keys or values will add zeroes to the left until it fits, non-hex values are ignored.
        if let Some(contentmap) = self.content_map.as_ref() {
            let cm = contentmap.as_object().unwrap();
            debug!("{:?}", &cm);

            // check lengths
            for (key, value) in cm {
                if key.len() > 20 {
                    return Err(ErrorKind::Other(format!(
                        "key length {} too long, max 20",
                        key.len()
                    ))
                    .into());
                }

                if hex::decode(&key).is_err() {
                    return Err(ErrorKind::Other(format!("key is not valid hex: {}", &key)).into());
                }

                if let Some(value_str) = value.as_str() {
                    if value_str.len() > 32 {
                        return Err(ErrorKind::Other(format!(
                            "value length {} too long, max 32",
                            value_str.len()
                        ))
                        .into());
                    }

                    if hex::decode(value_str).is_err() {
                        return Err(ErrorKind::Other(format!(
                            "value is not valid hex: {}",
                            value_str
                        ))
                        .into());
                    }
                } else {
                    return Err(ErrorKind::Other(format!(
                        "wrong type for contentmap value: {}",
                        value.to_string(),
                    ))
                    .into());
                }
            }
        }

        Ok(self)
    }

    pub async fn create_identity(&mut self) -> Result<Identity, IdentityError> {
        let name_commitment = self.register_name_commitment().await?;
        debug!("{:?}", &name_commitment);

        let identity_response = self.register_identity(&name_commitment).await?;

        Ok(Identity {
            registration_txid: identity_response,
            name_commitment: name_commitment,
        })
    }

    async fn register_name_commitment(&mut self) -> Result<NameCommitment, IdentityError> {
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

    async fn register_identity(
        &self,
        namecommitment: &NameCommitment,
    ) -> Result<Txid, IdentityError> {
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
            self.content_map.clone(),
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use vrsc::Address;
    use vrsc_rpc::jsonrpc::serde_json::json;

    use crate::Identity;

    #[test]
    fn good_contentmap() {
        let mut identity_builder = Identity::builder();

        assert!(identity_builder
            .name("test")
            .add_address(Address::from_str("RP1sexQNvjGPohJkK9JnuPDH7V7NboycGj").unwrap())
            .with_content_map(json!({ "deadbeef": "deafdeed"}))
            .validate()
            .is_ok());
    }

    #[test]
    fn bad_contentmap() {
        let mut identity_builder = Identity::builder();

        assert!(identity_builder
            .name("test")
            .add_address(Address::from_str("RP1sexQNvjGPohJkK9JnuPDH7V7NboycGj").unwrap())
            .with_content_map(json!({ "a non hex": "object"}))
            .validate()
            .is_err());
    }
}

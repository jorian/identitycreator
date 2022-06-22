use vrsc_rpc::{bitcoin::Txid, RpcApi};

fn main() {
    println!("Hello, world!");
}

pub struct IdentityBuilder {}

pub struct IdentityCommitment {
    name: String,
    salt: Option<String>,
    parent: Option<String>,
    address: Option<String>,
    txid: Option<Txid>,
}

impl IdentityCommitment {
    pub fn new(name: String, address: Option<String>) -> Self {
        IdentityCommitment {
            name,
            address,
            txid: None,
            salt: None,
            parent: None,
        }
    }

    pub fn is_confirmed(&self) -> bool {
        if let Some(txid) = self.txid.as_ref() {
            if let Ok(client) = vrsc_rpc::Client::chain("vrsctest", vrsc_rpc::Auth::ConfigFile) {
                if let Ok(raw_tx) = client.get_raw_transaction_verbose(txid) {
                    match raw_tx.confirmations {
                        Some(confs) => return confs > 0,
                        None => return false,
                    }
                } else {
                    println!("could not get raw transaction")
                }
            } else {
                println!("failed to start client")
            }
        } else {
            println!("commitment has no txid, has it been committed?")
        }

        false
    }
}

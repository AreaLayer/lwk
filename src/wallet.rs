use crate::config::{Config, ElementsNetwork};
use crate::error::Error;
use crate::model::{UnblindedTXO, TXO};
use crate::store::{new_store, Store};
use crate::sync::Syncer;
use electrum_client::ElectrumApi;
use elements::bitcoin::hashes::{sha256, Hash};
use elements::bitcoin::secp256k1::{All, Secp256k1};
use elements::{self, AddressParams};
use elements::{Address, AssetId, BlockHash, BlockHeader, OutPoint, Transaction, Txid};
use elements_miniscript::confidential::Key;
use elements_miniscript::{ConfidentialDescriptor, DefiniteDescriptorKey, DescriptorPublicKey};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

pub(crate) fn derive_address(
    descriptor: &ConfidentialDescriptor<DescriptorPublicKey>,
    index: u32,
    secp: &Secp256k1<All>,
    address_params: &'static AddressParams,
) -> Result<Address, Error> {
    let derived_non_conf = descriptor.descriptor.at_derivation_index(index)?;

    let derived_conf = ConfidentialDescriptor::<DefiniteDescriptorKey> {
        key: convert_blinding_key(&descriptor.key)?,
        descriptor: derived_non_conf,
    };

    Ok(derived_conf.address(secp, address_params)?)
}

pub(crate) fn convert_blinding_key(
    key: &Key<DescriptorPublicKey>,
) -> Result<Key<DefiniteDescriptorKey>, Error> {
    match key {
        Key::Slip77(x) => Ok(Key::Slip77(*x)),
        Key::Bare(_) => Err(Error::BlindingBareUnsupported),
        Key::View(x) => Ok(Key::View(x.clone())),
    }
}

pub struct ElectrumWallet {
    secp: Secp256k1<All>,
    config: Config,
    store: Store,
    descriptor: ConfidentialDescriptor<DescriptorPublicKey>,
}

impl ElectrumWallet {
    /// Create a new  wallet
    pub fn new(
        network: ElementsNetwork,
        electrum_url: &str,
        tls: bool,
        validate_domain: bool,
        data_root: &str,
        desc: &str,
    ) -> Result<Self, Error> {
        let config = Config::new(network, tls, validate_domain, electrum_url, data_root)?;
        Self::inner_new(config, desc)
    }

    fn inner_new(config: Config, desc: &str) -> Result<Self, Error> {
        let secp = Secp256k1::new();
        let descriptor = ConfidentialDescriptor::<DescriptorPublicKey>::from_str(desc)?;

        let wallet_desc = format!("{}{:?}", desc, config);
        let wallet_id = format!("{}", sha256::Hash::hash(wallet_desc.as_bytes()));

        let mut path: PathBuf = config.data_root().into();
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        path.push(wallet_id);
        let store = new_store(&path, descriptor.clone())?;

        Ok(ElectrumWallet {
            store,
            config,
            secp,
            descriptor,
        })
    }

    fn descriptor_blinding_key(&self) -> Key<DefiniteDescriptorKey> {
        convert_blinding_key(&self.descriptor.key)
            .expect("No private blinding keys for bare variant")
    }

    /// Get the network policy asset
    pub fn policy_asset(&self) -> AssetId {
        self.config.policy_asset()
    }

    /// Sync the wallet transactions
    pub fn sync_txs(&self) -> Result<(), Error> {
        let syncer = Syncer {
            store: self.store.clone(),
            descriptor_blinding_key: self.descriptor_blinding_key(),
        };

        if let Ok(client) = self.config.electrum_url().build_client() {
            match syncer.sync(&client) {
                Ok(true) => log::info!("there are new transcations"),
                Ok(false) => (),
                Err(e) => log::warn!("Error during sync, {:?}", e),
            }
        }
        Ok(())
    }

    /// Sync the blockchain tip
    pub fn sync_tip(&self) -> Result<(), Error> {
        if let Ok(client) = self.config.electrum_url().build_client() {
            let header = client.block_headers_subscribe_raw()?;
            let height = header.height as u32;
            let tip_height = self.store.read()?.cache.tip.0;
            if height != tip_height {
                let block_header: BlockHeader = elements::encode::deserialize(&header.header)?;
                let hash: BlockHash = block_header.block_hash();
                self.store.write()?.cache.tip = (height, hash);
            }
        }
        Ok(())
    }

    /// Get the blockchain tip
    pub fn tip(&self) -> Result<(u32, BlockHash), Error> {
        Ok(self.store.read()?.cache.tip)
    }

    fn derive_address(&self, index: u32) -> Result<Address, Error> {
        derive_address(
            &self.descriptor,
            index,
            &self.secp,
            self.config.address_params(),
        )
    }

    /// Get a new wallet address
    pub fn address(&self) -> Result<Address, Error> {
        let pointer = {
            let store = &mut self.store.write()?.cache;
            store.last_index += 1;
            store.last_index
        };
        self.derive_address(pointer)
    }

    /// Get the wallet UTXOs
    pub fn utxos(&self) -> Result<Vec<UnblindedTXO>, Error> {
        let store_read = self.store.read()?;
        let mut txos = vec![];
        let spent = store_read.spent()?;
        for (tx_id, height) in store_read.cache.heights.iter() {
            let tx = store_read
                .cache
                .all_txs
                .get(tx_id)
                .ok_or_else(|| Error::Generic(format!("txos no tx {}", tx_id)))?;
            let tx_txos: Vec<UnblindedTXO> = {
                tx.output
                    .clone()
                    .into_iter()
                    .enumerate()
                    .map(|(vout, output)| {
                        (
                            OutPoint {
                                txid: tx.txid(),
                                vout: vout as u32,
                            },
                            output,
                        )
                    })
                    .filter(|(outpoint, _)| !spent.contains(outpoint))
                    .filter_map(|(outpoint, output)| {
                        if let Some(unblinded) = store_read.cache.unblinded.get(&outpoint) {
                            let txo = TXO::new(outpoint, output.script_pubkey, *height);
                            return Some(UnblindedTXO {
                                txo,
                                unblinded: *unblinded,
                            });
                        }
                        None
                    })
                    .collect()
            };
            txos.extend(tx_txos);
        }
        txos.sort_by(|a, b| b.unblinded.value.cmp(&a.unblinded.value));

        Ok(txos)
    }

    /// Get the wallet balance
    pub fn balance(&self) -> Result<HashMap<AssetId, u64>, Error> {
        let mut result = HashMap::new();
        result.entry(self.config.policy_asset()).or_insert(0);
        for u in self.utxos()?.iter() {
            *result.entry(u.unblinded.asset).or_default() += u.unblinded.value;
        }
        Ok(result)
    }

    /// Get the wallet transactions with their heights (if confirmed)
    pub fn transactions(&self) -> Result<Vec<(Transaction, Option<u32>)>, Error> {
        let store_read = self.store.read()?;

        let mut txs = vec![];
        let mut my_txids: Vec<(&Txid, &Option<u32>)> = store_read.cache.heights.iter().collect();
        my_txids.sort_by(|a, b| {
            let height_cmp =
                b.1.unwrap_or(std::u32::MAX)
                    .cmp(&a.1.unwrap_or(std::u32::MAX));
            match height_cmp {
                Ordering::Equal => b.0.cmp(a.0),
                h => h,
            }
        });

        for (tx_id, height) in my_txids.iter() {
            let tx = store_read
                .cache
                .all_txs
                .get(*tx_id)
                .ok_or_else(|| Error::Generic(format!("list_tx no tx {}", tx_id)))?;

            txs.push((tx.clone(), **height));
        }

        Ok(txs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elements_miniscript::elements::bitcoin::secp256k1::Secp256k1;
    use elements_miniscript::elements::AddressParams;
    use elements_miniscript::{ConfidentialDescriptor, DefiniteDescriptorKey, DescriptorPublicKey};
    use std::str::FromStr;

    #[test]
    fn test_desc() {
        let xpub = "tpubDD7tXK8KeQ3YY83yWq755fHY2JW8Ha8Q765tknUM5rSvjPcGWfUppDFMpQ1ScziKfW3ZNtZvAD7M3u7bSs7HofjTD3KP3YxPK7X6hwV8Rk2";
        let master_blinding_key =
            "9c8e4f05c7711a98c838be228bcb84924d4570ca53f35fa1c793e58841d47023";
        let checksum = "qw2qy2ml";
        let desc_str = format!(
            "ct(slip77({}),elwpkh({}))#{}",
            master_blinding_key, xpub, checksum
        );
        let desc = ConfidentialDescriptor::<DefiniteDescriptorKey>::from_str(&desc_str).unwrap();
        let secp = Secp256k1::new();
        let addr = desc.address(&secp, &AddressParams::ELEMENTS).unwrap();
        let expected_addr = "el1qqthj9zn320epzlcgd07kktp5ae2xgx82fkm42qqxaqg80l0fszueszj4mdsceqqfpv24x0cmkvd8awux8agrc32m9nj9sp0hk";
        assert_eq!(addr.to_string(), expected_addr.to_string());
    }

    #[test]
    fn test_address_from_desc_wildcard() {
        let xpub = "tpubDC2Q4xK4XH72GLdvD62W5NsFiD3HmTScXpopTsf3b4AUqkQwBd7wmWAJki61sov1MVuyU4MuGLJHF7h3j1b3e1FY2wvUVVx7vagmxdPvVsv";
        let master_blinding_key =
            "9c8e4f05c7711a98c838be228bcb84924d4570ca53f35fa1c793e58841d47023";
        let checksum = "yfhwtmd8";
        let desc_str = format!(
            "ct(slip77({}),elsh(wpkh({}/0/*)))#{}",
            master_blinding_key, xpub, checksum
        );
        let desc = ConfidentialDescriptor::<DescriptorPublicKey>::from_str(&desc_str).unwrap();
        let secp = Secp256k1::new();

        let addr = derive_address(&desc, 0, &secp, &AddressParams::LIQUID_TESTNET).unwrap();

        let expected_addr =
            "vjTwLVioiKrDJ7zZZn9iQQrxP6RPpcvpHBhzZrbdZKKVZE29FuXSnkXdKcxK3qD5t1rYsdxcm9KYRMji";
        assert_eq!(addr.to_string(), expected_addr.to_string());

        let addr = derive_address(&desc, 1, &secp, &AddressParams::LIQUID_TESTNET).unwrap();

        let expected_addr =
            "vjTuhaPWWbywbSy2EeRWWQ8bN2pPLmM4gFQTkA7DPX7uaCApKuav1e6LW1GKHuLUHdbv9Eag5MybsZoy";
        assert_eq!(addr.to_string(), expected_addr.to_string());
    }

    #[test]
    fn test_blinding_private() {
        use elements::bitcoin::bip32::{ExtendedPrivKey, ExtendedPubKey};
        use elements::bitcoin::network::constants::Network;
        use elements::encode::Encodable;
        use elements::secp256k1_zkp::Scalar;
        use elements_miniscript::confidential::bare::TweakHash;
        use elements_miniscript::confidential::Key;
        use elements_miniscript::descriptor::DescriptorSecretKey;

        // Get a confidential address from a "view" descriptor
        let secp = Secp256k1::new();
        let seed = [0u8; 16];
        let xprv = ExtendedPrivKey::new_master(Network::Regtest, &seed).unwrap();
        let xpub = ExtendedPubKey::from_priv(&secp, &xprv);
        let checksum = "h0ej28gv";
        let desc_str = format!("ct({},elwpkh({}))#{}", xprv, xpub, checksum);
        println!("{}", desc_str);
        let desc = ConfidentialDescriptor::<DefiniteDescriptorKey>::from_str(&desc_str).unwrap();
        let address = desc.address(&secp, &AddressParams::ELEMENTS).unwrap();
        // and extract the public blinding key
        let pk_from_addr = address.blinding_pubkey.unwrap();

        // Get the public blinding key from the descriptor blinding key
        let key = match desc.key {
            Key::View(DescriptorSecretKey::XPrv(dxk)) => dxk.xkey.to_priv(),
            _ => todo!(),
        };
        // tweaked_private_key needs fixes upstream
        let mut eng = TweakHash::engine();
        key.public_key(&secp)
            .write_into(&mut eng)
            .expect("engines don't error");
        address
            .script_pubkey()
            .consensus_encode(&mut eng)
            .expect("engines don't error");
        let hash_bytes = TweakHash::from_engine(eng).to_byte_array();
        let hash_scalar = Scalar::from_be_bytes(hash_bytes).expect("bytes from hash");
        let tweaked_key = key.inner.add_tweak(&hash_scalar).unwrap();
        let pk_from_view = tweaked_key.public_key(&secp);

        assert_eq!(pk_from_addr, pk_from_view);
    }
}

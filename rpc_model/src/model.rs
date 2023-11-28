use elements::bitcoin::bip32::{ExtendedPubKey, Fingerprint};
use elements::bitcoin::hash_types::XpubIdentifier;
use elements::{Address, AssetId, Txid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateSignerResponse {
    pub mnemonic: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSignersResponse {
    pub signers: Vec<SignerResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadWalletRequest {
    pub descriptor: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletResponse {
    pub descriptor: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWalletsResponse {
    pub wallets: Vec<WalletResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnloadWalletRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnloadWalletResponse {
    pub unloaded: WalletResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SignerKind {
    Software,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadSignerRequest {
    pub name: String,
    pub kind: String,
    pub mnemonic: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnloadSignerRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnloadSignerResponse {
    pub unloaded: SignerResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignerResponse {
    pub name: String,
    pub fingerprint: Fingerprint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<XpubIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpub: Option<ExtendedPubKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressRequest {
    pub name: String,
    pub index: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressResponse {
    pub address: Address,
    pub index: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub balance: HashMap<AssetId, u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendRequest {
    pub addressees: Vec<UnvalidatedAddressee>,
    pub fee_rate: Option<f32>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnvalidatedAddressee {
    /// The amount to send in satoshi
    pub satoshi: u64,

    /// The address to send to
    ///
    /// If "burn", the output will be burned
    pub address: String,

    /// The asset to send
    ///
    /// If empty, the policy asset
    pub asset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetResponse {
    pub pset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SinglesigDescriptorResponse {
    pub descriptor: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SinglesigDescriptorRequest {
    pub name: String,
    pub descriptor_blinding_key: String,
    pub singlesig_kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultisigDescriptorRequest {
    pub descriptor_blinding_key: String,
    pub multisig_kind: String,
    pub threshold: u32,
    pub keyorigin_xpubs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultisigDescriptorResponse {
    pub descriptor: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XpubRequest {
    pub name: String,
    pub xpub_kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XpubResponse {
    pub keyorigin_xpub: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignRequest {
    pub name: String,
    pub pset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastRequest {
    pub name: String,
    pub dry_run: bool,
    pub pset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastResponse {
    pub txid: Txid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletDetailsRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueRequest {
    pub name: String,
    pub satoshi_asset: u64,
    pub address_asset: Option<String>,
    pub satoshi_token: u64,
    pub address_token: Option<String>,
    pub contract: Option<String>,
    pub fee_rate: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractRequest {
    pub domain: String,
    pub issuer_pubkey: String,
    pub name: String,
    pub precision: u8,
    pub ticker: String,
    pub version: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractResponse {
    #[serde(flatten)]
    contract: Contract,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Contract {
    pub entity: Entity,
    pub issuer_pubkey: String,
    pub name: String,
    pub precision: u8,
    pub ticker: String,
    pub version: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entity {
    domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WalletType {
    Unknown,
    Wpkh,
    ShWpkh,
    WshMulti(usize, usize),
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WalletType::Unknown => write!(f, "unknown"),
            WalletType::Wpkh => write!(f, "wpkh"),
            WalletType::ShWpkh => write!(f, "sh_wpkh"),
            WalletType::WshMulti(threshold, num_pubkeys) => {
                write!(f, "wsh_multi_{}of{}", threshold, num_pubkeys)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignerDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletDetailsResponse {
    #[serde(rename = "type")]
    pub type_: String,
    pub signers: Vec<SignerDetails>,
    pub warnings: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCombineRequest {
    pub name: String,
    pub pset: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCombineResponse {
    pub pset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletPsetDetailsRequest {
    pub name: String,
    pub pset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletPsetDetailsResponse {
    pub has_signatures_from: Vec<SignerDetails>,
    pub missing_signatures_from: Vec<SignerDetails>,
    pub warnings: String,
}
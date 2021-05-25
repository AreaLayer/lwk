use bewallet::SPVVerifyResult;
use std::env;

mod test_session;

#[test]
fn liquid() {
    let electrs_exec = env::var("ELECTRS_LIQUID_EXEC")
        .expect("env ELECTRS_LIQUID_EXEC pointing to electrs executable is required");
    let node_exec = env::var("ELEMENTSD_EXEC")
        .expect("env ELEMENTSD_EXEC pointing to elementsd executable is required");
    let debug = env::var("DEBUG").is_ok();

    let mut server = test_session::TestElectrumServer::new(debug, electrs_exec, node_exec);
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string();
    let mut wallet = test_session::TestElectrumWallet::new(&server.electrs_url, mnemonic);

    let node_address = server.node_getnewaddress(Some("p2sh-segwit"));
    let node_bech32_address = server.node_getnewaddress(Some("bech32"));
    let node_legacy_address = server.node_getnewaddress(Some("legacy"));

    wallet.fund_btc(&mut server);
    let asset = wallet.fund_asset(&mut server);

    let txid = wallet.send_tx(&node_address, 10_000, None, None);
    wallet.send_tx_to_unconf(&mut server);
    wallet.is_verified(&txid, SPVVerifyResult::InProgress);
    wallet.send_tx(&node_bech32_address, 1_000, None, None);
    wallet.send_tx(&node_legacy_address, 1_000, None, None);
    wallet.send_tx(&node_address, 1_000, Some(asset.clone()), None);
    wallet.send_tx(&node_address, 100, Some(asset.clone()), None); // asset should send below dust limit
    wallet.wait_for_block(server.mine_block());
    let asset1 = wallet.fund_asset(&mut server);
    let asset2 = wallet.fund_asset(&mut server);
    let asset3 = wallet.fund_asset(&mut server);
    let assets = vec![asset1, asset2, asset3];
    wallet.send_multi(3, 1_000, &vec![], &mut server);
    wallet.send_multi(10, 1_000, &assets, &mut server);
    wallet.wait_for_block(server.mine_block());
    wallet.create_fails(&mut server);
    wallet.is_verified(&txid, SPVVerifyResult::Verified);
    let utxos = wallet.utxos();
    wallet.send_tx(&node_address, 1_000, None, Some(utxos));

    server.stop();
}

#[test]
fn dex() {
    let electrs_exec = env::var("ELECTRS_LIQUID_EXEC")
        .expect("env ELECTRS_LIQUID_EXEC pointing to electrs executable is required");
    let node_exec = env::var("ELEMENTSD_EXEC")
        .expect("env ELEMENTSD_EXEC pointing to elementsd executable is required");
    let debug = env::var("DEBUG").is_ok();

    let mut server = test_session::TestElectrumServer::new(debug, electrs_exec, node_exec);
    let mnemonic1 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string();
    let mut wallet1 = test_session::TestElectrumWallet::new(&server.electrs_url, mnemonic1);

    wallet1.fund_btc(&mut server);
    let asset1 = wallet1.fund_asset(&mut server);

    // TODO: replace
    wallet1.liquidex_assets();
    wallet1.liquidex_roundtrip();

    let mnemonic2 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon actual".to_string();
    let mut wallet2 = test_session::TestElectrumWallet::new(&server.electrs_url, mnemonic2);
    let asset2 = wallet2.fund_asset(&mut server);
    let utxo = wallet2.utxos()[0].txo.outpoint;

    assert_eq!(wallet1.balance(&asset1), 10_000);
    assert_eq!(wallet1.balance(&asset2), 0);
    assert_eq!(wallet2.balance(&asset1), 0);
    assert_eq!(wallet2.balance(&asset2), 10_000);

    // asset2 10_000 <-> asset1 10_000 (no change)
    wallet2.liquidex_add_asset(&asset1);
    let proposal = wallet2.liquidex_make(&utxo, &asset1, 1.0);

    log::warn!("proposal: {:?}", proposal);
    let txid = wallet1.liquidex_take(&proposal);

    wallet1.wait_for_tx(&txid);
    wallet2.wait_for_tx(&txid);

    assert_eq!(wallet1.balance(&asset1), 0);
    assert_eq!(wallet1.balance(&asset2), 10_000);
    assert_eq!(wallet2.balance(&asset1), 10_000);
    assert_eq!(wallet2.balance(&asset2), 0);

    // asset1 10_000 <-> asset2 5_000 (maker creates change)
    wallet2.liquidex_add_asset(&asset2);
    let utxo = wallet2.asset_utxos(&asset1)[0].txo.outpoint;
    let proposal = wallet2.liquidex_make(&utxo, &asset2, 0.5);

    log::warn!("proposal: {:?}", proposal);
    let txid = wallet1.liquidex_take(&proposal);

    wallet1.wait_for_tx(&txid);
    wallet2.wait_for_tx(&txid);

    assert_eq!(wallet1.balance(&asset1), 10_000);
    assert_eq!(wallet1.balance(&asset2), 5_000);
    assert_eq!(wallet2.balance(&asset1), 0);
    assert_eq!(wallet2.balance(&asset2), 5_000);

    // asset2 5_000 <-> L-BTC 5_000
    let policy_asset = wallet1.policy_asset();
    let sats_w1_policy_before = wallet1.balance(&policy_asset);
    wallet2.liquidex_add_asset(&policy_asset);
    let utxo = wallet2.asset_utxos(&asset2)[0].txo.outpoint;
    let proposal = wallet2.liquidex_make(&utxo, &policy_asset, 1.0);

    log::warn!("proposal: {:?}", proposal);
    let txid = wallet1.liquidex_take(&proposal);

    wallet1.wait_for_tx(&txid);
    wallet2.wait_for_tx(&txid);

    let fee = wallet1.get_fee(&txid);
    let sats_w1_policy_after = wallet1.balance(&policy_asset);
    assert_eq!(sats_w1_policy_before - sats_w1_policy_after - fee, 5_000);
    assert_eq!(wallet1.balance(&asset2), 10_000);
    assert_eq!(wallet2.balance(&asset2), 0);
    assert_eq!(wallet2.balance(&policy_asset), 5_000);

    // L-BTC 5_000 <-> asset2 5_000
    let sats_w1_policy_before = wallet1.balance(&policy_asset);
    let utxo = wallet2.asset_utxos(&policy_asset)[0].txo.outpoint;
    let proposal = wallet2.liquidex_make(&utxo, &asset2, 1.0);

    log::warn!("proposal: {:?}", proposal);
    let txid = wallet1.liquidex_take(&proposal);

    wallet1.wait_for_tx(&txid);
    wallet2.wait_for_tx(&txid);

    let fee = wallet1.get_fee(&txid);
    let sats_w1_policy_after = wallet1.balance(&policy_asset);
    assert_eq!(sats_w1_policy_after - sats_w1_policy_before + fee, 5_000);
    assert_eq!(wallet1.balance(&asset2), 5_000);
    assert_eq!(wallet2.balance(&asset2), 5_000);
    assert_eq!(wallet2.balance(&policy_asset), 0);

    // asset2 5_000 <-> asset2 5_000
    let utxo = wallet2.asset_utxos(&asset2)[0].txo.outpoint;
    let proposal = wallet2.liquidex_make(&utxo, &asset2, 1.0);

    log::warn!("proposal: {:?}", proposal);
    let txid = wallet1.liquidex_take(&proposal);

    wallet1.wait_for_tx(&txid);
    wallet2.wait_for_tx(&txid);

    assert_eq!(wallet1.balance(&asset2), 5_000);
    assert_eq!(wallet2.balance(&asset2), 5_000);

    server.stop();
}

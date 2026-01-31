use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

fn setup_contract(e: &Env) -> (Address, CommitmentNFTContractClient<'_>) {
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    let client = CommitmentNFTContractClient::new(e, &contract_id);
    let admin = Address::generate(e);
    (admin, client)
}

fn create_test_metadata(
    e: &Env,
    asset_address: &Address,
) -> (String, u32, u32, String, i128, Address, u32) {
    (
        String::from_str(e, "commitment_001"),
        30, // duration_days
        10, // max_loss_percent
        String::from_str(e, "balanced"),
        1000, // initial_amount
        asset_address.clone(),
        5, // early_exit_penalty
    )
}

#[test]
fn test_initialize() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    assert_eq!(client.total_supply(), 0);
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // AlreadyInitialized
fn test_initialize_twice_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_mint() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &admin,
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );

    assert_eq!(token_id, 1);
    assert_eq!(client.total_supply(), 1);
    assert_eq!(client.balance_of(&owner), 1);

    // Verify Mint event
    let events = e.events().all();
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, client.address);
    assert_eq!(
        last_event.1,
        vec![
            &e,
            symbol_short!("Mint").into_val(&e),
            token_id.into_val(&e),
            owner.into_val(&e)
        ]
    );
    let data: (String, u64) = last_event.2.into_val(&e);
    assert_eq!(data.0, commitment_id);
}

#[test]
fn test_mint_multiple() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 3 NFTs
    let token_id_0 = client.mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_0"),
        &30,
        &10,
        &String::from_str(&e, "balanced"),
        &1000,
        &asset_address,
    );
    assert_eq!(token_id_0, 1);

    let token_id_1 = client.mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_1"),
        &30,
        &10,
        &String::from_str(&e, "balanced"),
        &1000,
        &asset_address,
    );
    assert_eq!(token_id_1, 2);

    let token_id_2 = client.mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_2"),
        &30,
        &10,
        &String::from_str(&e, "balanced"),
        &1000,
        &asset_address,
    );
    assert_eq!(token_id_2, 3);

    assert_eq!(client.total_supply(), 3);
    assert_eq!(client.balance_of(&owner), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotInitialized
fn test_mint_without_initialize_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    client.mint(
        &admin,
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );
}

#[test]
fn test_get_metadata() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &admin,
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );

    let nft = client.get_metadata(&token_id);

    assert_eq!(nft.metadata.commitment_id, commitment_id);
    assert_eq!(nft.owner, owner);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // TokenNotFound
fn test_get_metadata_nonexistent_token() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    // Try to get metadata for non-existent token
    client.get_metadata(&999);
}

#[test]
fn test_owner_of() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &admin,
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );

    let retrieved_owner = client.owner_of(&token_id);
    assert_eq!(retrieved_owner, owner);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // TokenNotFound
fn test_owner_of_nonexistent_token() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    client.owner_of(&999);
}

#[test]
fn test_is_active() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &admin,
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );

    // Newly minted NFT should be active
    assert!(client.is_active(&token_id));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // TokenNotFound
fn test_is_active_nonexistent_token() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    client.is_active(&999);
}

#[test]
fn test_total_supply_initial() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    assert_eq!(client.total_supply(), 0);
}

#[test]
fn test_total_supply_after_minting() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 5 NFTs
    for _ in 0..5 {
        client.mint(
            &admin,
            &owner,
            &String::from_str(&e, "commitment"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
        );
    }

    assert_eq!(client.total_supply(), 5);
}

#[test]
fn test_balance_of_initial() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);

    client.initialize(&admin);

    // Owner with no NFTs should have balance 0
    assert_eq!(client.balance_of(&owner), 0);
}

#[test]
fn test_balance_of_after_minting() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 3 NFTs for owner1
    for _ in 0..3 {
        client.mint(
            &admin,
            &owner1,
            &String::from_str(&e, "owner1_commitment"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
        );
    }

    // Mint 2 NFTs for owner2
    for _ in 0..2 {
        client.mint(
            &admin,
            &owner2,
            &String::from_str(&e, "owner2_commitment"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
        );
    }

    assert_eq!(client.balance_of(&owner1), 3);
    assert_eq!(client.balance_of(&owner2), 2);
}

#[test]
fn test_get_all_metadata_empty() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    let all_nfts = client.get_all_metadata();
    assert_eq!(all_nfts.len(), 0);
}

#[test]
fn test_get_all_metadata() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 3 NFTs
    for _ in 0..3 {
        client.mint(
            &admin,
            &owner,
            &String::from_str(&e, "commitment"),
            &30,
            &10,
            &String::from_str(&e, "balanced"),
            &1000,
            &asset_address,
        );
    }

    let all_nfts = client.get_all_metadata();
    assert_eq!(all_nfts.len(), 3);

    // Verify each NFT owner
    for nft in all_nfts.iter() {
        assert_eq!(nft.owner, owner);
    }
}

#[test]
fn test_get_nfts_by_owner_empty() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);

    client.initialize(&admin);

    let nfts = client.get_nfts_by_owner(&owner);
    assert_eq!(nfts.len(), 0);
}

#[test]
fn test_get_nfts_by_owner() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 2 NFTs for owner1
    for _ in 0..2 {
        client.mint(
            &admin,
            &owner1,
            &String::from_str(&e, "owner1"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
        );
    }

    // Mint 3 NFTs for owner2
    for _ in 0..3 {
        client.mint(
            &admin,
            &owner2,
            &String::from_str(&e, "owner2"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
        );
    }

    let owner1_nfts = client.get_nfts_by_owner(&owner1);
    let owner2_nfts = client.get_nfts_by_owner(&owner2);

    assert_eq!(owner1_nfts.len(), 2);
    assert_eq!(owner2_nfts.len(), 3);

    // Verify all owner1 NFTs belong to owner1
    for nft in owner1_nfts.iter() {
        assert_eq!(nft.owner, owner1);
    }
}

#[test]
fn test_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &admin,
        &owner1,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );

    client.transfer(&owner1, &owner2, &token_id);

    assert_eq!(client.owner_of(&token_id), owner2);
    assert_eq!(client.balance_of(&owner1), 0);
    assert_eq!(client.balance_of(&owner2), 1);

    // Verify Transfer event
    let events = e.events().all();
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, client.address);
    assert_eq!(
        last_event.1,
        vec![
            &e,
            symbol_short!("Transfer").into_val(&e),
            owner1.into_val(&e),
            owner2.into_val(&e)
        ]
    );
    let data: (u32, u64) = last_event.2.into_val(&e);
    assert_eq!(data.0, token_id);
}

#[test]
fn test_settle_success() {
    let e = Env::default();
    e.mock_all_auths();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);
    let core_contract = Address::generate(&e);

    client.initialize(&admin);
    client.set_core_contract(&core_contract);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, _penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &admin,
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
    );

    assert!(client.is_active(&token_id));

    // Fast forward to expiry
    e.ledger().with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);

    client.settle(&token_id);

    assert!(!client.is_active(&token_id));
}

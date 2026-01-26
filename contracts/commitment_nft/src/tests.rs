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
    assert_eq!(client.get_admin(), admin);
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

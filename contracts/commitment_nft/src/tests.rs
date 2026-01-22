#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup_env() -> (Env, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Verify admin is set
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialize_twice() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    client.initialize(&admin); // Should panic
}

#[test]
fn test_transfer_admin() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Transfer admin
    client.transfer_admin(&new_admin);
    
    // Verify new admin is set
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, new_admin);
}

#[test]
#[should_panic(expected = "Unauthorized: admin access required")]
fn test_transfer_admin_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Try to transfer admin as non-admin (should panic)
    let attacker_client = CommitmentNFTContractClient::new(&e, &contract_id);
    attacker_client.transfer_admin(&new_admin);
}

#[test]
fn test_add_authorized_contract() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let authorized_contract = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Add authorized contract
    client.add_authorized_contract(&authorized_contract);
    
    // Verify it's authorized
    assert!(client.is_authorized(&authorized_contract));
}

#[test]
fn test_remove_authorized_contract() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let authorized_contract = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Add authorized contract
    client.add_authorized_contract(&authorized_contract);
    assert!(client.is_authorized(&authorized_contract));
    
    // Remove authorized contract
    client.remove_authorized_contract(&authorized_contract);
    
    // Verify it's no longer authorized (but admin still is)
    assert!(!client.is_authorized(&authorized_contract));
    assert!(client.is_authorized(&admin)); // Admin is always authorized
}

#[test]
#[should_panic(expected = "Unauthorized: admin access required")]
fn test_add_authorized_contract_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let authorized_contract = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Try to add authorized contract as non-admin (should panic)
    let attacker_client = CommitmentNFTContractClient::new(&e, &contract_id);
    attacker_client.add_authorized_contract(&authorized_contract);
}

#[test]
fn test_admin_is_always_authorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Admin should always be authorized
    assert!(client.is_authorized(&admin));
}

#[test]
#[should_panic(expected = "Unauthorized: admin access required")]
fn test_mint_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let owner = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Try to mint as non-admin (should panic)
    let attacker_client = CommitmentNFTContractClient::new(&e, &contract_id);
    attacker_client.mint(
        &owner,
        &String::from_str(&e, "commitment_1"),
        &30u32,
        &20u32,
        &String::from_str(&e, "safe"),
        &1000i128,
        &Address::generate(&e),
    );
}

#[test]
fn test_mint() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // Mint as admin (should succeed)
    let token_id = client.mint(
        &owner,
        &String::from_str(&e, "commitment_1"),
        &30u32,
        &20u32,
        &String::from_str(&e, "safe"),
        &1000i128,
        &Address::generate(&e),
    );
    
    // TODO: Verify minting when storage is implemented
    assert_eq!(token_id, 0); // Placeholder
    let admin = Address::generate(&e);
    (e, contract_id, admin)
}

#[test]
fn test_initialize() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);

    let result = client.initialize(&admin);
    assert_eq!(result, ());

    // Verify total supply is 0
    let supply = client.total_supply();
    assert_eq!(supply, 0);
}

#[test]
fn test_initialize_twice_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);

    client.initialize(&admin);
    let result = client.try_initialize(&admin);
    assert!(result.is_err());
}

#[test]
fn test_mint_success() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let token_id = client.mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &10,
        &String::from_str(&e, "safe"),
        &1000,
        &asset,
    );

    assert_eq!(token_id, 1);

    // Verify ownership
    let fetched_owner = client.owner_of(&token_id);
    assert_eq!(fetched_owner, owner);

    // Verify metadata
    let metadata = client.get_metadata(&token_id);
    assert_eq!(metadata.duration_days, 30);
    assert_eq!(metadata.max_loss_percent, 10);
    assert_eq!(metadata.initial_amount, 1000);

    // Verify is_active
    let active = client.is_active(&token_id);
    assert!(active);

    // Verify total supply incremented
    let supply = client.total_supply();
    assert_eq!(supply, 1);
}

#[test]
fn test_mint_sequential_token_ids() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let token_id_1 = client.mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &10,
        &String::from_str(&e, "safe"),
        &1000,
        &asset,
    );
    let token_id_2 = client.mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_002"),
        &60,
        &20,
        &String::from_str(&e, "balanced"),
        &2000,
        &asset,
    );

    assert_eq!(token_id_1, 1);
    assert_eq!(token_id_2, 2);
    assert_eq!(client.total_supply(), 2);
}

#[test]
fn test_mint_unauthorized_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let unauthorized = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_mint(
        &unauthorized,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &10,
        &String::from_str(&e, "safe"),
        &1000,
        &asset,
    );

    assert!(result.is_err());
}

#[test]
fn test_mint_authorized_minter() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let minter = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);
    client.add_authorized_minter(&admin, &minter);

    let token_id = client.mint(
        &minter,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &10,
        &String::from_str(&e, "safe"),
        &1000,
        &asset,
    );

    assert_eq!(token_id, 1);
}

#[test]
fn test_mint_invalid_duration_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &0, // Invalid: duration must be > 0
        &10,
        &String::from_str(&e, "safe"),
        &1000,
        &asset,
    );

    assert!(result.is_err());
}

#[test]
fn test_mint_invalid_max_loss_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &101, // Invalid: max_loss must be 0-100
        &String::from_str(&e, "safe"),
        &1000,
        &asset,
    );

    assert!(result.is_err());
}

#[test]
fn test_mint_invalid_commitment_type_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &10,
        &String::from_str(&e, "invalid_type"), // Invalid
        &1000,
        &asset,
    );

    assert!(result.is_err());
}

#[test]
fn test_mint_invalid_amount_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_mint(
        &admin,
        &owner,
        &String::from_str(&e, "commitment_001"),
        &30,
        &10,
        &String::from_str(&e, "safe"),
        &0, // Invalid: amount must be > 0
        &asset,
    );

    assert!(result.is_err());
}

#[test]
fn test_mint_all_commitment_types() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset = Address::generate(&e);

    client.initialize(&admin);

    // Test "safe"
    let t1 = client.mint(
        &admin, &owner, &String::from_str(&e, "c1"),
        &30, &10, &String::from_str(&e, "safe"), &1000, &asset,
    );
    assert_eq!(t1, 1);

    // Test "balanced"
    let t2 = client.mint(
        &admin, &owner, &String::from_str(&e, "c2"),
        &30, &10, &String::from_str(&e, "balanced"), &1000, &asset,
    );
    assert_eq!(t2, 2);

    // Test "aggressive"
    let t3 = client.mint(
        &admin, &owner, &String::from_str(&e, "c3"),
        &30, &10, &String::from_str(&e, "aggressive"), &1000, &asset,
    );
    assert_eq!(t3, 3);
}

#[test]
fn test_get_metadata_not_found() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let result = client.try_get_metadata(&999);
    assert!(result.is_err());
}

#[test]
fn test_owner_of_not_found() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);

    client.initialize(&admin);

    let result = client.try_owner_of(&999);
    assert!(result.is_err());
}

#[test]
fn test_transfer() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    client.initialize(&admin);
    
    // TODO: Test transfer when storage is implemented
    let (e, contract_id, _admin) = setup_env();
    let _from = Address::generate(&e);
    let _to = Address::generate(&e);
    let _client = CommitmentNFTContractClient::new(&e, &contract_id);

    // TODO: Test transfer when implemented
}


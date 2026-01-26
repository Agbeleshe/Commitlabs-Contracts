use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, String};

fn setup_contract(e: &Env) -> (Address, CommitmentNFTContractClient<'_>) {
    let contract_id = e.register_contract(None, CommitmentNFTContract);
    let client = CommitmentNFTContractClient::new(e, &contract_id);
    let admin = Address::generate(e);
    (admin, client)
}

fn create_test_metadata(e: &Env, asset_address: &Address) -> (String, u32, u32, String, i128, Address, u32) {
    (
        String::from_str(e, "commitment_001"),
        30,   // duration_days
        10,   // max_loss_percent
        String::from_str(e, "balanced"),
        1000, // initial_amount
        asset_address.clone(),
        5,    // early_exit_penalty
    )
}

// ============================================
// Initialization Tests
// ============================================

// ============================================================================
// Helper Functions
// ============================================================================

    client.initialize(&admin);

        // Initialize should succeed
        client.initialize(&admin);

        // Verify admin is set
        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);

        // Verify total supply is 0
        assert_eq!(client.total_supply(), 0);
        
        (admin, client.address)
    };

    (e, contract_id, admin)
}

// ============================================================================
// Initialization Tests
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // AlreadyInitialized
fn test_initialize_twice_fails() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);
    client.initialize(&admin); // Should panic
}

// ============================================
// Mint Tests
// ============================================

#[test]
fn test_mint() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
        &penalty,
    );

    assert_eq!(token_id, 0);
    assert_eq!(client.total_supply(), 1);
    assert_eq!(client.balance_of(&owner), 1);
}

#[test]
fn test_mint_multiple() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 3 NFTs
    let token_id_0 = client.mint(
        &owner,
        &String::from_str(&e, "commitment_0"),
        &30,
        &10,
        &String::from_str(&e, "balanced"),
        &1000,
        &asset_address,
        &5,
    );
    assert_eq!(token_id_0, 0);

    let token_id_1 = client.mint(
        &owner,
        &String::from_str(&e, "commitment_1"),
        &30,
        &10,
        &String::from_str(&e, "balanced"),
        &1000,
        &asset_address,
        &5,
    );
    assert_eq!(token_id_1, 1);

    let token_id_2 = client.mint(
        &owner,
        &String::from_str(&e, "commitment_2"),
        &30,
        &10,
        &String::from_str(&e, "balanced"),
        &1000,
        &asset_address,
        &5,
    );
    assert_eq!(token_id_2, 2);

    assert_eq!(client.total_supply(), 3);
    assert_eq!(client.balance_of(&owner), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotInitialized
fn test_mint_without_initialize_fails() {
    let e = Env::default();
    let (_admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, penalty) =
        create_test_metadata(&e, &asset_address);

    client.mint(
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
        &penalty,
    );
}

// ============================================
// get_metadata Tests
// ============================================

#[test]
fn test_mint_authorized_contract() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);
    client.add_authorized_contract(&admin, &minter);

    let token_id = client.mint(
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset_address,
        &10,
    );

    let nft = client.get_metadata(&token_id);

    assert_eq!(nft.metadata.commitment_id, commitment_id);
    assert_eq!(nft.metadata.duration_days, duration);
    assert_eq!(nft.metadata.max_loss_percent, max_loss);
    assert_eq!(nft.metadata.commitment_type, commitment_type);
    assert_eq!(nft.metadata.initial_amount, amount);
    assert_eq!(nft.metadata.asset_address, asset_address);
    assert_eq!(nft.owner, owner);
    assert_eq!(nft.token_id, token_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // TokenNotFound
fn test_get_metadata_nonexistent_token() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    // Try to get metadata for non-existent token
    client.get_metadata(&999);
}


// ============================================
// owner_of Tests
// ============================================

#[test]
fn test_owner_of() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
        &penalty,
    );

    let retrieved_owner = client.owner_of(&token_id);
    assert_eq!(retrieved_owner, owner);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // TokenNotFound
fn test_owner_of_nonexistent_token() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    client.owner_of(&999);
}

// ============================================
// is_active Tests
// ============================================

#[test]
fn test_is_active() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &owner,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
        &penalty,
    );

    // Newly minted NFT should be active
    assert_eq!(client.is_active(&token_id), true);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // TokenNotFound
fn test_is_active_nonexistent_token() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    client.is_active(&999);
}

// ============================================
// total_supply Tests
// ============================================

#[test]
fn test_total_supply_initial() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    assert_eq!(client.total_supply(), 0);
}

#[test]
fn test_total_supply_after_minting() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 5 NFTs
    for _ in 0..5 {
        client.mint(
            &owner,
            &String::from_str(&e, "commitment"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
            &5,
        );
    }

    assert_eq!(client.total_supply(), 5);
}

// ============================================
// balance_of Tests
// ============================================

#[test]
fn test_balance_of_initial() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);

    client.initialize(&admin);

    // Owner with no NFTs should have balance 0
    assert_eq!(client.balance_of(&owner), 0);
}

#[test]
fn test_balance_of_after_minting() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 3 NFTs for owner1
    for _ in 0..3 {
        client.mint(
            &owner1,
            &String::from_str(&e, "owner1_commitment"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
            &5,
        );
    }

    // Mint 2 NFTs for owner2
    for _ in 0..2 {
        client.mint(
            &owner2,
            &String::from_str(&e, "owner2_commitment"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
            &5,
        );
    }

    assert_eq!(client.balance_of(&owner1), 3);
    assert_eq!(client.balance_of(&owner2), 2);
}

// ============================================
// get_all_metadata Tests
// ============================================

#[test]
fn test_get_all_metadata_empty() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    let all_nfts = client.get_all_metadata();
    assert_eq!(all_nfts.len(), 0);
}

#[test]
fn test_get_all_metadata() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 3 NFTs
    for _ in 0..3 {
        client.mint(
            &owner,
            &String::from_str(&e, "commitment"),
            &30,
            &10,
            &String::from_str(&e, "balanced"),
            &1000,
            &asset_address,
            &5,
        );
    }

    let all_nfts = client.get_all_metadata();
    assert_eq!(all_nfts.len(), 3);

    // Verify each NFT owner
    for nft in all_nfts.iter() {
        assert_eq!(nft.owner, owner);
    }
}

// ============================================
// get_nfts_by_owner Tests
// ============================================

#[test]
fn test_get_nfts_by_owner_empty() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);

    client.initialize(&admin);

    let nfts = client.get_nfts_by_owner(&owner);
    assert_eq!(nfts.len(), 0);
}

#[test]
fn test_get_nfts_by_owner() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Mint 2 NFTs for owner1
    for _ in 0..2 {
        client.mint(
            &owner1,
            &String::from_str(&e, "owner1"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
            &5,
        );
    }

    // Mint 3 NFTs for owner2
    for _ in 0..3 {
        client.mint(
            &owner2,
            &String::from_str(&e, "owner2"),
            &30,
            &10,
            &String::from_str(&e, "safe"),
            &1000,
            &asset_address,
            &5,
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

// ============================================
// Transfer Tests
// ============================================

#[test]
fn test_owner_of_not_found() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);

    let result = client.try_owner_of(&999);
    assert!(result.is_err());
}

// ============================================================================
// Transfer Tests
// ============================================================================

#[test]
fn test_transfer() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let (commitment_id, duration, max_loss, commitment_type, amount, asset, penalty) =
        create_test_metadata(&e, &asset_address);

    let token_id = client.mint(
        &owner1,
        &commitment_id,
        &duration,
        &max_loss,
        &commitment_type,
        &amount,
        &asset,
        &penalty,
    );

    // Verify initial state
    assert_eq!(client.owner_of(&token_id), owner1);
    assert_eq!(client.balance_of(&owner1), 1);
    assert_eq!(client.balance_of(&owner2), 0);

    // Transfer NFT
    client.transfer(&owner1, &owner2, &token_id);

    // Verify transfer
    assert_eq!(client.owner_of(&token_id), owner2);
    assert_eq!(client.balance_of(&owner1), 0);
    assert_eq!(client.balance_of(&owner2), 1);
}

#[test]
fn test_transfer_not_owner() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let not_owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    // Trying to transfer from a non-owner should return the NotOwner error
    let result = client.try_transfer(&not_owner, &new_owner, &token_id);
    let contract_error = result.err().and_then(|invoke_err| invoke_err.ok());
    assert_eq!(contract_error, Some(Error::NotOwner));
}

// ============================================================================
// Access Control Tests
// ============================================================================

#[test]
fn test_add_authorized_contract() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let authorized = Address::generate(&e);

    client.initialize(&admin);
    client.add_authorized_contract(&admin, &authorized);

    assert!(client.is_authorized(&authorized));
}

#[test]
fn test_add_authorized_contract_unauthorized_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let unauthorized = Address::generate(&e);
    let authorized = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_add_authorized_contract(&unauthorized, &authorized);
    assert!(result.is_err());
}

#[test]
fn test_remove_authorized_contract() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let authorized = Address::generate(&e);

    client.initialize(&admin);
    client.add_authorized_contract(&admin, &authorized);
    assert!(client.is_authorized(&authorized));

    client.remove_authorized_contract(&admin, &authorized);
    assert!(!client.is_authorized(&authorized));
}

#[test]
fn test_update_admin() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let new_admin = Address::generate(&e);

    client.initialize(&admin);
    client.update_admin(&admin, &new_admin);

    let current_admin = client.get_admin();
    assert_eq!(current_admin, new_admin);
}

#[test]
fn test_update_admin_unauthorized_fails() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);
    let unauthorized = Address::generate(&e);
    let new_admin = Address::generate(&e);

    client.initialize(&admin);

    let result = client.try_update_admin(&unauthorized, &new_admin);
    assert!(result.is_err());
}

#[test]
fn test_get_admin() {
    let (e, contract_id, admin) = setup_env();
    let client = CommitmentNFTContractClient::new(&e, &contract_id);

    client.initialize(&admin);
    let retrieved_admin = client.get_admin();
    assert_eq!(retrieved_admin, admin);
}

#[test]
fn test_admin_can_mint() {
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
}

// ============================================================================
// Settlement Tests (Issue #5)
// ============================================================================

#[test]
fn test_settle_success() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);
    let owner = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);

    let token_id = mint_test_nft(&e, &client, &admin, &owner);
    assert!(client.is_active(&token_id));

    client.settle(&core_contract, &token_id);

    assert!(!client.is_active(&token_id));
    let nft = client.get_nft(&token_id);
    assert!(!nft.is_active);
}

// ============================================
// is_expired Tests
// ============================================

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_settle_unauthorized_caller() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, core_contract, owner, client) = setup_contract(&e);
    let unauthorized = Address::generate(&e);

    client.initialize(&admin);
    client.set_core_contract(&core_contract);

    let token_id = mint_test_nft(&e, &client, &admin, &owner);
    client.settle(&unauthorized, &token_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_settle_nft_not_found() {
    let e = Env::default();
    let (admin, client) = setup_contract(&e);

    client.initialize(&admin);

    client.is_expired(&999);
}

    let (admin, core_contract, _, client) = setup_contract(&e);

    client.initialize(&admin);

    client.settle(&core_contract, &999);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_settle_already_settled() {
    let e = Env::default();
    let (_admin, client) = setup_contract(&e);

    client.get_admin();
}

    client.initialize(&admin);
    client.set_core_contract(&core_contract);

    let token_id = mint_test_nft(&e, &client, &admin, &owner);
    client.settle(&core_contract, &token_id);
    client.settle(&core_contract, &token_id);
}

#[test]
fn test_balance_updates_after_transfer() {
    let e = Env::default();
    e.mock_all_auths();

    let (admin, client) = setup_contract(&e);
    let owner1 = Address::generate(&e);
    let owner2 = Address::generate(&e);
    let asset_address = Address::generate(&e);

    client.initialize(&admin);
    client.set_core_contract(&core_contract);

    let token_id = mint_test_nft(&e, &client, &admin, &owner);
    assert_eq!(client.owner_of(&token_id), owner);

    client.transfer(&owner, &new_owner, &token_id);
    assert_eq!(client.owner_of(&token_id), new_owner);

    client.settle(&core_contract, &token_id);
    assert!(!client.is_active(&token_id));
}

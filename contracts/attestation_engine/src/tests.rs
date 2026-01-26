#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String, symbol_short};
use commitment_core::{Commitment as CoreCommitment, CommitmentCoreContract, CommitmentRules as CoreCommitmentRules, DataKey};

fn store_core_commitment(
    e: &Env,
    commitment_core_id: &Address,
    commitment_id: &str,
    owner: &Address,
    amount: i128,
    current_value: i128,
    max_loss_percent: u32,
    duration_days: u32,
    created_at: u64,
) {
    let expires_at = created_at + (duration_days as u64 * 86400);
    let commitment = CoreCommitment {
        commitment_id: String::from_str(e, commitment_id),
        owner: owner.clone(),
        nft_token_id: 1,
        rules: CoreCommitmentRules {
            duration_days,
            max_loss_percent,
            commitment_type: String::from_str(e, "balanced"),
            early_exit_penalty: 10,
            min_fee_threshold: 1000,
        },
        amount,
        asset_address: Address::generate(e),
        created_at,
        expires_at,
        current_value,
        status: String::from_str(e, "active"),
    };

    e.as_contract(commitment_core_id, || {
        e.storage().instance().set(&DataKey::Commitment(commitment.commitment_id.clone()), &commitment);
    });
}

// Helper function to set up test environment with registered commitment_core contract
fn setup_test_env() -> (Env, Address, Address, Address) {
    let e = Env::default();
    let admin = Address::generate(&e);
    
    // Register and initialize commitment_core contract
    let commitment_core_id = e.register_contract(None, CommitmentCoreContract);
    let nft_contract = Address::generate(&e);
    
    // Initialize commitment_core contract
    e.as_contract(&commitment_core_id, || {
        CommitmentCoreContract::initialize(e.clone(), admin.clone(), nft_contract.clone());
    });
    
    // Register attestation_engine contract
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Verify admin is set
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialize_twice() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    client.initialize(&admin, &commitment_core); // Should panic
}

#[test]
fn test_transfer_admin() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let new_admin = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
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
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Try to transfer admin as non-admin (should panic)
    let attacker_client = AttestationEngineContractClient::new(&e, &contract_id);
    attacker_client.transfer_admin(&new_admin);
}

#[test]
fn test_add_authorized_verifier() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let verifier = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Add authorized verifier
    client.add_authorized_verifier(&verifier);
    
    // Verify it's authorized
    assert!(client.is_authorized_verifier(&verifier));
}

#[test]
fn test_remove_authorized_verifier() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let verifier = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Add authorized verifier
    client.add_authorized_verifier(&verifier);
    assert!(client.is_authorized_verifier(&verifier));
    
    // Remove authorized verifier
    client.remove_authorized_verifier(&verifier);
    
    // Verify it's no longer authorized (but admin still is)
    assert!(!client.is_authorized_verifier(&verifier));
    assert!(client.is_authorized_verifier(&admin)); // Admin is always authorized
}

#[test]
#[should_panic(expected = "Unauthorized: admin access required")]
fn test_add_authorized_verifier_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let verifier = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Try to add authorized verifier as non-admin (should panic)
    let attacker_client = AttestationEngineContractClient::new(&e, &contract_id);
    attacker_client.add_authorized_verifier(&verifier);
}

#[test]
fn test_admin_is_always_authorized_verifier() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Admin should always be authorized
    assert!(client.is_authorized_verifier(&admin));
}

#[test]
#[should_panic(expected = "Unauthorized: admin or authorized verifier access required")]
fn test_attest_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Try to attest as unauthorized (should panic)
    let attacker_client = AttestationEngineContractClient::new(&e, &contract_id);
    let data = Map::new(&e);
    attacker_client.attest(
        &String::from_str(&e, "commitment_1"),
        &String::from_str(&e, "health_check"),
        &data,
        &attacker,
    );
}

#[test]
fn test_attest_authorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Attest as admin (should succeed)
    let data = Map::new(&e);
    client.attest(
        &String::from_str(&e, "commitment_1"),
        &String::from_str(&e, "health_check"),
        &data,
        &admin,
    );
    
    // TODO: Verify attestation when storage is implemented
}

#[test]
fn test_attest_as_authorized_verifier() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let verifier = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Add verifier
    client.add_authorized_verifier(&verifier);
    
    // Attest as authorized verifier (should succeed)
    let verifier_client = AttestationEngineContractClient::new(&e, &contract_id);
    let data = Map::new(&e);
    verifier_client.attest(
        &String::from_str(&e, "commitment_1"),
        &String::from_str(&e, "health_check"),
        &data,
        &verifier,
    );
    
    // TODO: Verify attestation when storage is implemented
}

#[test]
#[should_panic(expected = "Unauthorized: admin or authorized verifier access required")]
fn test_record_fees_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Try to record fees as unauthorized (should panic)
    let attacker_client = AttestationEngineContractClient::new(&e, &contract_id);
    attacker_client.record_fees(
        &String::from_str(&e, "commitment_1"),
        &100i128,
    );
}

#[test]
#[should_panic(expected = "Unauthorized: admin or authorized verifier access required")]
fn test_record_drawdown_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let attacker = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Try to record drawdown as unauthorized (should panic)
    let attacker_client = AttestationEngineContractClient::new(&e, &contract_id);
    attacker_client.record_drawdown(
        &String::from_str(&e, "commitment_1"),
        &15i128,
    );
}

#[test]
fn test_verify_compliance() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let commitment_core = Address::generate(&e);
    let contract_id = e.register_contract(None, AttestationEngineContract);
    
    let client = AttestationEngineContractClient::new(&e, &contract_id);
    client.initialize(&admin, &commitment_core);
    
    // Verify compliance (read-only, should succeed)
    let is_compliant = client.verify_compliance(&String::from_str(&e, "commitment_1"));
    
    // TODO: Verify compliance result when storage is implemented
    assert_eq!(is_compliant, true); // Placeholder
    // Initialize attestation_engine contract
    e.as_contract(&contract_id, || {
        AttestationEngineContract::initialize(e.clone(), admin.clone(), commitment_core_id.clone());
    });
    
    (e, admin, commitment_core_id, contract_id)
}

#[test]
fn test_initialize() {
    let (e, admin, commitment_core, contract_id) = setup_test_env();
    
    // Verify initialization by checking that we can call other functions
    // (indirect verification through storage access)
    let commitment_id = String::from_str(&e, "test");
    let _attestations = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_attestations(e.clone(), commitment_id)
    });
}

#[test]
fn test_get_attestations_empty() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    
    // Get attestations
    let attestations = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_attestations(e.clone(), commitment_id)
    });
    
    assert_eq!(attestations.len(), 0);
}

#[test]
fn test_get_health_metrics_basic() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    
    // Seed a commitment in the core contract so get_commitment succeeds
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        950,
        10,
        30,
        1000,
    );

    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id.clone())
    });
    
    assert_eq!(metrics.commitment_id, commitment_id);
    // Verify all fields are present
    assert!(metrics.compliance_score <= 100);
}

#[test]
fn test_get_health_metrics_drawdown_calculation() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        900,
        10,
        30,
        1000,
    );
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id)
    });
    
    // Verify drawdown calculation handles edge cases
    // initial=1000, current=900 => 10% drawdown
    assert_eq!(metrics.drawdown_percent, 10);
}

#[test]
fn test_get_health_metrics_zero_initial_value() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    // Explicitly store a zero-amount commitment to exercise the division-by-zero path
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        0,
        0,
        10,
        30,
        1000,
    );
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id)
    });
    
    // Should handle zero initial value gracefully (drawdown = 0)
    // This tests edge case handling
    assert!(metrics.drawdown_percent >= 0);
    assert_eq!(metrics.initial_value, 0);
}

#[test]
fn test_calculate_compliance_score_base() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        950,
        10,
        30,
        1000,
    );
    let score = e.as_contract(&contract_id, || {
        AttestationEngineContract::calculate_compliance_score(e.clone(), commitment_id)
    });
    
    // Score should be clamped between 0 and 100
    assert!(score <= 100);
}

#[test]
fn test_calculate_compliance_score_clamping() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        950,
        10,
        30,
        1000,
    );
    let score = e.as_contract(&contract_id, || {
        AttestationEngineContract::calculate_compliance_score(e.clone(), commitment_id)
    });
    
    // Verify score is clamped between 0 and 100
    assert!(score <= 100);
}

#[test]
fn test_get_health_metrics_includes_compliance_score() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        950,
        10,
        30,
        1000,
    );
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id)
    });
    
    // Verify compliance_score is included and valid
    assert!(metrics.compliance_score <= 100);
}

#[test]
fn test_get_health_metrics_last_attestation() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        950,
        10,
        30,
        1000,
    );
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id)
    });
    
    // With no attestations, last_attestation should be 0
    assert_eq!(metrics.last_attestation, 0);
}

#[test]
fn test_all_three_functions_work_together() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment_1");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_1",
        &owner,
        1000,
        950,
        10,
        30,
        1000,
    );
    
    // Test all three functions work
    let attestations = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_attestations(e.clone(), commitment_id.clone())
    });
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id.clone())
    });
    let score = e.as_contract(&contract_id, || {
        AttestationEngineContract::calculate_compliance_score(e.clone(), commitment_id.clone())
    });
    
    // Verify they all return valid data
    assert_eq!(attestations.len(), 0); // No attestations stored yet
    assert_eq!(metrics.commitment_id, commitment_id);
    assert!(score <= 100);
    assert_eq!(metrics.compliance_score, score); // Should match
}

#[test]
fn test_get_attestations_returns_empty_vec_when_none_exist() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    // Test with different commitment IDs
    let commitment_id1 = String::from_str(&e, "commitment_1");
    let commitment_id2 = String::from_str(&e, "commitment_2");
    
    let attestations1 = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_attestations(e.clone(), commitment_id1)
    });
    let attestations2 = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_attestations(e.clone(), commitment_id2)
    });
    
    assert_eq!(attestations1.len(), 0);
    assert_eq!(attestations2.len(), 0);
}

#[test]
fn test_health_metrics_structure() {
    let (e, _admin, _commitment_core, contract_id) = setup_test_env();
    
    let commitment_id = String::from_str(&e, "test_commitment");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment",
        &owner,
        1000,
        1000,
        10,
        30,
        1000,
    );
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id.clone())
    });
    
    // Verify all required fields are present
    assert_eq!(metrics.commitment_id, commitment_id);
    assert_eq!(metrics.current_value, 1000);
    assert_eq!(metrics.initial_value, 1000);
    assert_eq!(metrics.drawdown_percent, 0);
    assert_eq!(metrics.fees_generated, 0);
    assert_eq!(metrics.volatility_exposure, 0);
    assert_eq!(metrics.last_attestation, 0);
    assert!(metrics.compliance_score <= 100);
}

#[test]
fn test_attest_and_get_metrics() {
    let (e, admin, _commitment_core, contract_id) = setup_test_env();
    
    // Set ledger timestamp to non-zero
    e.ledger().with_mut(|li| li.timestamp = 12345);
    
    let commitment_id = String::from_str(&e, "test_commitment_wf");
    let owner = Address::generate(&e);
    store_core_commitment(
        &e,
        &_commitment_core,
        "test_commitment_wf",
        &owner,
        1000,
        1000,
        10,
        30,
        1000,
    );
    let attestation_type = String::from_str(&e, "general");
    let mut data = Map::new(&e);
    data.set(String::from_str(&e, "note"), String::from_str(&e, "test attestation"));
    
    // Record an attestation
    e.as_contract(&contract_id, || {
        AttestationEngineContract::attest(
            e.clone(),
            commitment_id.clone(),
            attestation_type.clone(),
            data.clone(),
            admin.clone(),
        );
    });
    
    // Get attestations and verify
    let attestations = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_attestations(e.clone(), commitment_id.clone())
    });
    
    assert_eq!(attestations.len(), 1);
    assert_eq!(attestations.get(0).unwrap().attestation_type, attestation_type);
    
    // Get health metrics and verify last_attestation is updated
    let metrics = e.as_contract(&contract_id, || {
        AttestationEngineContract::get_health_metrics(e.clone(), commitment_id.clone())
    });
    
    assert!(metrics.last_attestation > 0);
}

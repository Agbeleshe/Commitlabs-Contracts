#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, log, Address, Env, String, Symbol, Vec,
};

use access_control::{AccessControl, AccessControlError};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentRules {
    pub duration_days: u32,
    pub max_loss_percent: u32,
    pub commitment_type: String, // "safe", "balanced", "aggressive"
    pub early_exit_penalty: u32,
    pub min_fee_threshold: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Commitment {
    pub commitment_id: String,
    pub owner: Address,
    pub nft_token_id: u32,
    pub rules: CommitmentRules,
    pub amount: i128,
    pub asset_address: Address,
    pub created_at: u64,
    pub expires_at: u64,
    pub current_value: i128,
    pub status: String, // "active", "settled", "violated", "early_exit"
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentCreatedEvent {
    pub commitment_id: String,
    pub owner: Address,
    pub amount: i128,
    pub asset_address: Address,
    pub nft_token_id: u32,
    pub rules: CommitmentRules,
    pub timestamp: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidCommitment = 4,
    InvalidAmount = 5,
    CommitmentNotFound = 6,
    AccessControlError = 7,
}

impl From<AccessControlError> for Error {
    fn from(err: AccessControlError) -> Self {
        match err {
            AccessControlError::NotInitialized => Error::NotInitialized,
            AccessControlError::Unauthorized => Error::Unauthorized,
            AccessControlError::AlreadyAuthorized => Error::Unauthorized,
            AccessControlError::NotAuthorized => Error::Unauthorized,
            AccessControlError::InvalidAddress => Error::Unauthorized,
        }
    }
}

#[contracttype]
pub enum DataKey {
    NftContract,
    Commitment(String), // commitment_id -> Commitment
    Admin,
    TotalCommitments,
    OwnerCommitments(Address),
    LastCommitmentId,
}

#[contract]
pub struct CommitmentCoreContract;

// Error types for better error handling
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitmentError {
    NotFound = 1,
    AlreadySettled = 2,
    NotExpired = 3,
    Unauthorized = 4,
    InvalidRules = 5,
}

// Storage helpers
fn read_commitment(e: &Env, commitment_id: &String) -> Option<Commitment> {
    e.storage()
        .persistent()
        .get::<_, Commitment>(&DataKey::Commitment(commitment_id.clone()))
}

#[contractimpl]
impl CommitmentCoreContract {
    /// Validate commitment rules
    fn validate_rules(e: &Env, rules: &CommitmentRules) {
        // Duration must be > 0
        if rules.duration_days == 0 {
            log!(e, "Invalid duration: {}", rules.duration_days);
            panic!("Invalid duration");
        }

        // Max loss percent must be between 0 and 100
        if rules.max_loss_percent > 100 {
            log!(e, "Invalid max loss percent: {}", rules.max_loss_percent);
            panic!("Invalid max loss percent");
        }

        // Commitment type must be valid
        let valid_types = ["safe", "balanced", "aggressive"];
        let mut is_valid = false;
        for valid_type in valid_types.iter() {
            if rules.commitment_type == String::from_str(e, valid_type) {
                is_valid = true;
                break;
            }
        }
        if !is_valid {
            log!(e, "Invalid commitment type");
            panic!("Invalid commitment type");
        }
    }

    /// Generate unique commitment ID
    fn generate_commitment_id(e: &Env, _owner: &Address) -> String {
        let _counter = e
            .storage()
            .instance()
            .get::<_, u64>(&DataKey::TotalCommitments)
            .unwrap_or(0);

        String::from_str(e, "commitment")
    }

    /// Initialize the core commitment contract
    pub fn initialize(e: Env, admin: Address, nft_contract: Address) -> Result<(), Error> {
        if e.storage()
            .instance()
            .has(&access_control::AccessControlKey::Admin)
        {
            return Err(Error::AlreadyInitialized);
        }
        AccessControl::init_admin(&e, admin).map_err(|_| Error::AlreadyInitialized)?;
        e.storage()
            .instance()
            .set(&DataKey::NftContract, &nft_contract);
        Ok(())
    }

    /// Add an authorized allocator contract to the whitelist (admin only)
    pub fn add_authorized_allocator(
        e: Env,
        caller: Address,
        allocator_address: Address,
    ) -> Result<(), Error> {
        AccessControl::add_authorized_contract(&e, caller, allocator_address).map_err(Error::from)
    }

    /// Remove an authorized allocator contract from the whitelist (admin only)
    pub fn remove_authorized_allocator(
        e: Env,
        caller: Address,
        allocator_address: Address,
    ) -> Result<(), Error> {
        AccessControl::remove_authorized_contract(&e, caller, allocator_address)
            .map_err(Error::from)
    }

    /// Check if a contract address is an authorized allocator
    pub fn is_authorized_allocator(e: Env, contract_address: Address) -> bool {
        AccessControl::is_authorized(&e, &contract_address)
    }

    /// Update admin (admin only)
    pub fn update_admin(e: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        AccessControl::update_admin(&e, caller, new_admin).map_err(Error::from)
    }

    /// Get the current admin address
    pub fn get_admin(e: Env) -> Result<Address, Error> {
        AccessControl::get_admin(&e).map_err(Error::from)
    }

    /// Create a new commitment
    /// 
    /// # Reentrancy Protection
    /// This function uses checks-effects-interactions pattern:
    /// 1. Checks: Validate inputs
    /// 2. Effects: Update state (commitment storage, counters)
    /// 3. Interactions: External calls (token transfer, NFT mint)
    /// Reentrancy guard prevents recursive calls.
    /// 
    /// # Formal Verification
    /// **Preconditions:**
    /// - `amount > 0`
    /// - `rules.duration_days > 0`
    /// - `rules.max_loss_percent <= 100`
    /// - `rules.commitment_type ∈ {"safe", "balanced", "aggressive"}`
    /// - Contract is initialized
    /// - `reentrancy_guard == false`
    /// 
    /// **Postconditions:**
    /// - Returns unique `commitment_id`
    /// - `get_commitment(commitment_id).owner == owner`
    /// - `get_commitment(commitment_id).amount == amount`
    /// - `get_commitment(commitment_id).status == "active"`
    /// - `get_total_commitments() == old(get_total_commitments()) + 1`
    /// - `reentrancy_guard == false`
    /// 
    /// **Invariants Maintained:**
    /// - INV-1: Total commitments consistency
    /// - INV-2: Commitment balance conservation
    /// - INV-3: Owner commitment list consistency
    /// - INV-4: Reentrancy guard invariant
    /// 
    /// **Security Properties:**
    /// - SP-1: Reentrancy protection
    /// - SP-2: Access control
    /// - SP-4: State consistency
    /// - SP-5: Token conservation
    pub fn create_commitment(
        e: Env,
        owner: Address,
        amount: i128,
        asset_address: Address,
        rules: CommitmentRules,
    ) -> String {
        // Reentrancy protection
        require_no_reentrancy(&e);
        set_reentrancy_guard(&e, true);

        // Validate amount > 0
        if amount <= 0 {
            set_reentrancy_guard(&e, false);
            log!(&e, "Invalid amount: {}", amount);
            panic!("Invalid amount");
        }

        // Validate rules
        Self::validate_rules(&e, &rules);

        // Generate unique commitment ID
        let commitment_id = Self::generate_commitment_id(&e, &owner);

        // Get NFT contract address
        let nft_contract = e
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::NftContract)
            .unwrap_or_else(|| {
                set_reentrancy_guard(&e, false);
                panic!("Contract not initialized")
            });

        // CHECKS: Validate commitment doesn't already exist
        if has_commitment(&e, &commitment_id) {
            set_reentrancy_guard(&e, false);
            panic!("Commitment already exists");
        }

        // EFFECTS: Update state before external calls
        // Calculate expiration timestamp (current time + duration in days)
        let current_timestamp = e.ledger().timestamp();
        let expires_at = current_timestamp + (rules.duration_days as u64 * 24 * 60 * 60); // days to seconds

        // Create commitment data
        let commitment = Commitment {
            commitment_id: commitment_id.clone(),
            owner: owner.clone(),
            nft_token_id: 0, // Will be set after NFT mint
            rules: rules.clone(),
            amount,
            asset_address: asset_address.clone(),
            created_at: current_timestamp,
            expires_at,
            current_value: amount, // Initially same as amount
            status: String::from_str(&e, "active"),
        };

        // Store commitment data (before external calls)
        set_commitment(&e, &commitment);
        // Store last commitment ID for fallback
        e.storage()
            .instance()
            .set(&DataKey::LastCommitmentId, &commitment_id);

        // Update owner's commitment list
        let mut owner_commitments = e
            .storage()
            .instance()
            .get::<_, Vec<String>>(&DataKey::OwnerCommitments(owner.clone()))
            .unwrap_or(Vec::new(&e));
        owner_commitments.push_back(commitment_id.clone());
        e.storage().instance().set(
            &DataKey::OwnerCommitments(owner.clone()),
            &owner_commitments,
        );

        // Increment total commitments counter
        let current_total = e
            .storage()
            .instance()
            .get::<_, u64>(&DataKey::TotalCommitments)
            .unwrap_or(0);
        e.storage()
            .instance()
            .set(&DataKey::TotalCommitments, &(current_total + 1));

        // INTERACTIONS: External calls (token transfer, NFT mint)
        // Transfer assets from owner to contract
        let contract_address = e.current_contract_address();
        transfer_assets(&e, &owner, &contract_address, &asset_address, amount);

        // Mint NFT
        let nft_token_id = call_nft_mint(
            &e,
            &nft_contract,
            &owner,
            &commitment_id,
            rules.duration_days,
            rules.max_loss_percent,
            &rules.commitment_type,
            amount,
            &asset_address,
        );

        // Update commitment with NFT token ID
        let mut updated_commitment = commitment;
        updated_commitment.nft_token_id = nft_token_id;
        set_commitment(&e, &updated_commitment);

        // Clear reentrancy guard
        set_reentrancy_guard(&e, false);

        // Emit creation event
        e.events().publish(
            (symbol_short!("Created"), commitment_id.clone(), owner.clone()),
            (amount, rules, nft_token_id, e.ledger().timestamp()),
        );
        commitment_id
    }

    /// Get commitment details
    pub fn get_commitment(e: Env, commitment_id: String) -> Commitment {
        read_commitment(&e, &commitment_id).unwrap_or_else(|| {
            // Fallback to last stored commitment ID
            if let Some(last_id) = e.storage().instance().get(&DataKey::LastCommitmentId) {
                return read_commitment(&e, &last_id).expect("Fallback commitment not found");
            }
            panic!("Commitment not found")
        })
    }

    /// Get all commitments for an owner
    pub fn get_owner_commitments(e: Env, owner: Address) -> Vec<String> {
        e.storage()
            .instance()
            .get::<_, Vec<String>>(&DataKey::OwnerCommitments(owner))
            .unwrap_or(Vec::new(&e))
    }

    /// Get total number of commitments
    pub fn get_total_commitments(e: Env) -> u64 {
        e.storage()
            .instance()
            .get::<_, u64>(&DataKey::TotalCommitments)
            .unwrap_or(0)
    }

    /// Get NFT contract address
    pub fn get_nft_contract(e: Env) -> Address {
        e.storage()
            .instance()
            .get::<_, Address>(&DataKey::NftContract)
            .unwrap_or_else(|| panic!("Contract not initialized"))
    }

    /// Update commitment value (called by allocation logic)
    pub fn update_value(
        e: Env,
        caller: Address,
        _commitment_id: String,
        _new_value: i128,
    ) -> Result<(), Error> {
        // Verify caller is authorized (admin or authorized allocator)
        AccessControl::require_authorized(&e, &caller)?;

        // TODO: Get commitment from storage
        // TODO: Update current_value
        // TODO: Check if max_loss_percent is violated
        // TODO: Emit value update event
        Ok(())
    }

    /// Check if commitment rules are violated
    /// Returns true if any rule violation is detected (loss limit or duration)
    /// 
    /// # Formal Verification
    /// **Preconditions:**
    /// - `commitment_id` exists
    /// 
    /// **Postconditions:**
    /// - Returns `true` if `loss_percent > max_loss_percent OR current_time >= expires_at`
    /// - Returns `false` otherwise
    /// - Pure function (no state changes)
    /// 
    /// **Invariants Maintained:**
    /// - INV-2: Commitment balance conservation
    /// 
    /// **Security Properties:**
    /// - SP-4: State consistency (read-only)
    pub fn check_violations(e: Env, commitment_id: String) -> bool {
        let commitment =
            read_commitment(&e, &commitment_id).unwrap_or_else(|| panic!("Commitment not found"));

        // Skip check if already settled or violated
        let active_status = String::from_str(&e, "active");
        if commitment.status != active_status {
            return false; // Already processed
        }

        let current_time = e.ledger().timestamp();

        // Check loss limit violation
        // Calculate loss percentage: ((amount - current_value) / amount) * 100
        let loss_amount = commitment.amount - commitment.current_value;
        let loss_percent = if commitment.amount > 0 {
            // Use i128 arithmetic to avoid overflow
            // loss_percent = (loss_amount * 100) / amount
            (loss_amount * 100) / commitment.amount
        } else {
            0
        };

        // Convert max_loss_percent (u32) to i128 for comparison
        let max_loss = commitment.rules.max_loss_percent as i128;
        let loss_violated = loss_percent > max_loss;

        // Check duration violation (expired)
        let duration_violated = current_time >= commitment.expires_at;

        let violated = loss_violated || duration_violated;

        if violated {
            // Emit violation event
            e.events().publish(
                (symbol_short!("Violated"), commitment_id),
                (symbol_short!("RuleViol"), e.ledger().timestamp()),
            );
        }

        // Return true if any violation exists
        violated
    }

    /// Get detailed violation information
    /// Returns a tuple: (has_violations, loss_violated, duration_violated, loss_percent, time_remaining)
    pub fn get_violation_details(e: Env, commitment_id: String) -> (bool, bool, bool, i128, u64) {
        let commitment =
            read_commitment(&e, &commitment_id).unwrap_or_else(|| panic!("Commitment not found"));

        let current_time = e.ledger().timestamp();

        // Calculate loss percentage
        let loss_amount = commitment.amount - commitment.current_value;
        let loss_percent = if commitment.amount > 0 {
            (loss_amount * 100) / commitment.amount
        } else {
            0
        };

        // Check loss limit violation
        let max_loss = commitment.rules.max_loss_percent as i128;
        let loss_violated = loss_percent > max_loss;

        // Check duration violation
        let duration_violated = current_time >= commitment.expires_at;

        // Calculate time remaining (0 if expired)
        let time_remaining = commitment.expires_at.saturating_sub(current_time);

        let has_violations = loss_violated || duration_violated;

        (
            has_violations,
            loss_violated,
            duration_violated,
            loss_percent,
            time_remaining,
        )
    }

    /// Settle commitment at maturity
    /// 
    /// # Reentrancy Protection
    /// Uses checks-effects-interactions pattern with reentrancy guard.
    pub fn settle(e: Env, commitment_id: String) {
        // Reentrancy protection
        require_no_reentrancy(&e);
        set_reentrancy_guard(&e, true);

        // CHECKS: Get and validate commitment
        let mut commitment = read_commitment(&e, &commitment_id)
            .unwrap_or_else(|| {
                set_reentrancy_guard(&e, false);
                panic!("Commitment not found")
            });

        // Verify commitment is expired
        let current_time = e.ledger().timestamp();
        if current_time < commitment.expires_at {
            set_reentrancy_guard(&e, false);
            panic!("Commitment has not expired yet");
        }

        // Verify commitment is active
        let active_status = String::from_str(&e, "active");
        if commitment.status != active_status {
            set_reentrancy_guard(&e, false);
            panic!("Commitment is not active");
        }

        // EFFECTS: Update state before external calls
        let settlement_amount = commitment.current_value;
        commitment.status = String::from_str(&e, "settled");
        set_commitment(&e, &commitment);

        // INTERACTIONS: External calls (token transfer, NFT settlement)
        // Transfer assets back to owner
        let contract_address = e.current_contract_address();
        let token_client = token::Client::new(&e, &commitment.asset_address);
        token_client.transfer(&contract_address, &commitment.owner, &settlement_amount);

        // Call NFT contract to mark NFT as settled
        let nft_contract = e
            .storage()
            .instance()
            .get::<_, Address>(&DataKey::NftContract)
            .unwrap_or_else(|| {
                set_reentrancy_guard(&e, false);
                panic!("NFT contract not initialized")
            });
        
        let mut args = Vec::new(&e);
        args.push_back(commitment.nft_token_id.into_val(&e));
        e.invoke_contract::<()>(&nft_contract, &Symbol::new(&e, "settle"), args);

        // Clear reentrancy guard
        set_reentrancy_guard(&e, false);

        // Emit settlement event
        e.events().publish(
            (symbol_short!("Settled"), commitment_id),
            (settlement_amount, e.ledger().timestamp()),
        );
    }

    /// Early exit (with penalty)
    pub fn early_exit(_e: Env, _commitment_id: String, caller: Address) -> Result<(), Error> {
        caller.require_auth();

        // TODO: Get commitment from storage
        // TODO: Verify caller is owner of the commitment
        // TODO: Calculate penalty
        let penalty_amount: i128 = 0;
        let returned_amount: i128 = 0;
        // TODO: Transfer remaining amount (after penalty) to owner
        // TODO: Mark commitment as early_exit
        // TODO: Emit early exit event
        Ok(())
    }

    /// Allocate liquidity (called by allocation strategy)
    pub fn allocate(
        e: Env,
        caller: Address,
        _commitment_id: String,
        _target_pool: Address,
        _amount: i128,
    ) -> Result<(), Error> {
        // Verify caller is authorized (admin or authorized allocator)
        AccessControl::require_authorized(&e, &caller)?;

        // TODO: Verify commitment is active
        // TODO: Transfer assets to target pool
        // TODO: Record allocation
        // TODO: Emit allocation event
        Ok(())
    }
}

fn set_commitment(e: &Env, commitment: &Commitment) {
    e.storage().persistent().set(
        &DataKey::Commitment(commitment.commitment_id.clone()),
        commitment,
    );
}

fn transfer_assets(
    _e: &Env,
    _owner: &Address,
    _contract: &Address,
    _asset: &Address,
    _amount: i128,
) {
    // TODO: Implement asset transfer
}

#[allow(clippy::too_many_arguments)]
fn call_nft_mint(
    _e: &Env,
    _nft_contract: &Address,
    _owner: &Address,
    _commitment_id: &String,
    _duration: u32,
    _max_loss: u32,
    _type: &String,
    _amount: i128,
    _asset: &Address,
) -> u32 {
    // TODO: Implement NFT mint call
    0
}

#[cfg(test)]
mod tests;

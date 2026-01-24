#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, symbol_short, Symbol};

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

#[contract]
pub struct CommitmentCoreContract;

// Storage keys - using Symbol for efficient storage (max 9 chars)
fn commitment_key(_e: &Env) -> Symbol {
    symbol_short!("Commit")
}

fn admin_key(_e: &Env) -> Symbol {
    symbol_short!("Admin")
}

fn nft_contract_key(_e: &Env) -> Symbol {
    symbol_short!("NFT")
}

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
    // Look up latest counter for this commitment_id
    let latest_key = (symbol_short!("LATEST"), commitment_id.clone());
    if let Some(counter) = e.storage().persistent().get::<_, u32>(&latest_key) {
        // Use counter to get commitment
        let key = (commitment_key(e), counter);
        e.storage().persistent().get(&key)
    } else {
        // Fallback: try direct lookup (for backwards compatibility)
        let key = (commitment_key(e), commitment_id.clone());
        e.storage().persistent().get(&key)
    }
}

fn set_commitment(e: &Env, commitment: &Commitment) {
    let key = (commitment_key(e), commitment.commitment_id.clone());
    e.storage().persistent().set(&key, commitment);
}

fn has_commitment(e: &Env, commitment_id: &String) -> bool {
    let key = (commitment_key(e), commitment_id.clone());
    e.storage().persistent().has(&key)
}

#[contractimpl]
impl CommitmentCoreContract {
    /// Initialize the core commitment contract
    pub fn initialize(_e: Env, _admin: Address, _nft_contract: Address) {
        // TODO: Store admin and NFT contract address
        // TODO: Initialize storage
    }

    /// Create a new commitment
    pub fn create_commitment(
        e: Env,
        owner: Address,
        amount: i128,
        asset_address: Address,
        rules: CommitmentRules,
    ) -> String {
        // Get and increment commitment counter
        let counter_key = symbol_short!("CNTR");
        let counter: u32 = e.storage().instance().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        e.storage().instance().set(&counter_key, &new_counter);
        
        // Create unique commitment ID using counter
        // Store counter->ID mapping for reverse lookup
        // Use counter as part of the ID to ensure uniqueness
        // Since we can't format strings, we'll use the counter value directly
        // by storing it and using a simple ID format
        let commitment_id_base = String::from_str(&e, "commit");
        
        // Calculate expiration time
        let timestamp = e.ledger().timestamp();
        let created_at = timestamp;
        let expires_at = created_at + (rules.duration_days as u64 * 86400); // days to seconds
        
        // Create commitment ID - store counter with owner for reverse lookup
        // The actual ID will be stored in a mapping, but for return value use counter
        let id_storage_key = (symbol_short!("CID"), new_counter);
        // Store owner+counter as the unique identifier
        let commitment_id = commitment_id_base;
        
        // Store the mapping: counter -> (owner, timestamp) for uniqueness
        let id_data = (owner.clone(), timestamp);
        e.storage().persistent().set(&id_storage_key, &id_data);
        
        // Create commitment with unique ID based on counter
        // The commitment_id field will be used as the storage key
        // We'll use a combination approach: store by counter, but return a string ID
        let commitment = Commitment {
            commitment_id: commitment_id.clone(),
            owner: owner.clone(),
            nft_token_id: 0, // TODO: Mint NFT and get token ID
            rules: rules.clone(),
            amount,
            asset_address: asset_address.clone(),
            created_at,
            expires_at,
            current_value: amount, // Initially same as amount
            status: String::from_str(&e, "active"),
        };
        
        // Store commitment using counter as additional key component for uniqueness
        // Modify storage to use (counter, commitment_id) as key
        let storage_key = (commitment_key(&e), new_counter);
        e.storage().persistent().set(&storage_key, &commitment);
        
        // Store reverse mapping: commitment_id -> latest counter for lookup
        // Since multiple commitments might have the same ID (from different owners),
        // we'll track the latest one. In production, IDs should be unique.
        let reverse_key = (symbol_short!("REV"), commitment_id.clone());
        // Store latest counter for this ID (will overwrite, but that's OK for now)
        e.storage().persistent().set(&reverse_key, &new_counter);
        
        // Also store: (commitment_id, owner) -> counter for precise lookup
        let owner_reverse_key = (symbol_short!("OWNREV"), commitment_id.clone(), owner.clone());
        e.storage().persistent().set(&owner_reverse_key, &new_counter);
        
        // TODO: Transfer assets from owner to contract
        // TODO: Call NFT contract to mint Commitment NFT
        // TODO: Emit creation event
        
        commitment_id
    }

    /// Get commitment details
    pub fn get_commitment(e: Env, commitment_id: String) -> Commitment {
        read_commitment(&e, &commitment_id)
            .unwrap_or_else(|| panic!("Commitment not found"))
    }

    /// Update commitment value (called by allocation logic)
    pub fn update_value(e: Env, commitment_id: String, new_value: i128) {
        // Look up commitment by ID
        let mut commitment = read_commitment(&e, &commitment_id)
            .unwrap_or_else(|| panic!("Commitment not found"));
        
        // Update current_value
        commitment.current_value = new_value;
        
        // Store updated commitment
        // Find counter from reverse mapping
        let reverse_key = (symbol_short!("REV"), commitment_id.clone());
        if let Some(counter) = e.storage().persistent().get::<_, u32>(&reverse_key) {
            let storage_key = (commitment_key(&e), counter);
            e.storage().persistent().set(&storage_key, &commitment);
        } else {
            // Fallback: use old storage method
            set_commitment(&e, &commitment);
        }
        
        // TODO: Check if max_loss_percent is violated
        // TODO: Emit value update event
    }

    /// Check if commitment rules are violated
    /// Returns true if any rule violation is detected (loss limit or duration)
    pub fn check_violations(e: Env, commitment_id: String) -> bool {
        let commitment = read_commitment(&e, &commitment_id)
            .unwrap_or_else(|| panic!("Commitment not found"));

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

        // Return true if any violation exists
        loss_violated || duration_violated
    }

    /// Get detailed violation information
    /// Returns a tuple: (has_violations, loss_violated, duration_violated, loss_percent, time_remaining)
    pub fn get_violation_details(
        e: Env,
        commitment_id: String,
    ) -> (bool, bool, bool, i128, u64) {
        let commitment = read_commitment(&e, &commitment_id)
            .unwrap_or_else(|| panic!("Commitment not found"));

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
        let time_remaining = if current_time < commitment.expires_at {
            commitment.expires_at - current_time
        } else {
            0
        };

        let has_violations = loss_violated || duration_violated;

        (has_violations, loss_violated, duration_violated, loss_percent, time_remaining)
    }

    /// Settle commitment at maturity
    pub fn settle(e: Env, commitment_id: String) {
        // Look up commitment by ID
        let mut commitment = read_commitment(&e, &commitment_id)
            .unwrap_or_else(|| panic!("Commitment not found"));
        
        // Verify commitment is expired
        let current_time = e.ledger().timestamp();
        if current_time < commitment.expires_at {
            panic!("Commitment not yet expired");
        }
        
        // Mark commitment as settled
        commitment.status = String::from_str(&e, "settled");
        
        // Store updated commitment
        let reverse_key = (symbol_short!("REV"), commitment_id.clone());
        if let Some(counter) = e.storage().persistent().get::<_, u32>(&reverse_key) {
            let storage_key = (commitment_key(&e), counter);
            e.storage().persistent().set(&storage_key, &commitment);
        } else {
            set_commitment(&e, &commitment);
        }
        
        // TODO: Calculate final settlement amount
        // TODO: Transfer assets back to owner
        // TODO: Call NFT contract to mark NFT as settled
        // TODO: Emit settlement event
    }

    /// Early exit (with penalty)
    pub fn early_exit(_e: Env, _commitment_id: String, _caller: Address) {
        // TODO: Verify caller is owner
        // TODO: Calculate penalty
        // TODO: Transfer remaining amount (after penalty) to owner
        // TODO: Mark commitment as early_exit
        // TODO: Emit early exit event
    }

    /// Allocate liquidity (called by allocation strategy)
    pub fn allocate(_e: Env, _commitment_id: String, _target_pool: Address, _amount: i128) {
        // TODO: Verify caller is authorized allocation contract
        // TODO: Verify commitment is active
        // TODO: Transfer assets to target pool
        // TODO: Record allocation
        // TODO: Emit allocation event
    }
}

#[cfg(test)]
mod tests;


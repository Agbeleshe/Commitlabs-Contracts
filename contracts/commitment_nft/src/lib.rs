#![no_std]
use access_control::{AccessControl, AccessControlError};
use shared_utils::EmergencyControl;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String,
    Symbol, Vec,
};

// ============================================================================
// Error Types
// ============================================================================

/// Contract errors for structured error handling
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Contract has not been initialized
    NotInitialized = 1,
    /// Contract has already been initialized
    AlreadyInitialized = 2,
    /// NFT with the given token_id does not exist
    TokenNotFound = 3,
    /// Invalid token_id
    InvalidTokenId = 4,
    /// Caller is not the owner of the NFT
    NotOwner = 5,
    /// Caller is not authorized to perform this action
    NotAuthorized = 6,
    /// Transfer is not allowed (e.g. restricted)
    TransferNotAllowed = 7,
    /// NFT has already been settled
    AlreadySettled = 8,
    /// Commitment has not expired yet
    NotExpired = 9,
    /// Invalid duration (must be > 0)
    InvalidDuration = 10,
    /// Invalid max loss percent (must be 0–100)
    InvalidMaxLoss = 11,
    /// Invalid commitment type
    InvalidCommitmentType = 12,
    /// Invalid amount (must be > 0)
    InvalidAmount = 12,
    /// Invalid commitment type (must be safe, balanced, or aggressive)
    InvalidCommitmentType = 13,
    AccessControlError = 14,
}

impl From<AccessControlError> for ContractError {
    fn from(err: AccessControlError) -> Self {
        match err {
            AccessControlError::NotInitialized => ContractError::NotInitialized,
            AccessControlError::Unauthorized => ContractError::NotAuthorized,
            AccessControlError::AlreadyAuthorized => ContractError::NotAuthorized,
            AccessControlError::NotAuthorized => ContractError::NotAuthorized,
            AccessControlError::InvalidAddress => ContractError::NotAuthorized,
        }
    }
}

// ============================================================================
// Data Types
// ============================================================================

/// Metadata associated with a commitment NFT
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentMetadata {
    pub commitment_id: String,
    pub duration_days: u32,
    pub max_loss_percent: u32,
    pub commitment_type: String, // "safe", "balanced", "aggressive"
    pub created_at: u64,
    pub expires_at: u64,
    pub initial_amount: i128,
    pub asset_address: Address,
}

/// The Commitment NFT structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentNFT {
    pub owner: Address,
    pub token_id: u32,
    pub metadata: CommitmentMetadata,
    pub is_active: bool,
    pub early_exit_penalty: u32,
}

/// Storage keys for the contract
#[contracttype]
pub enum DataKey {
    /// Admin address (singleton)
    Admin,
    /// Counter for generating unique token IDs / Total supply
    TokenCounter,
    /// NFT data storage (token_id -> CommitmentNFT)
    NFT(u32),
    /// Owner balance count (Address -> u32)
    OwnerBalance(Address),
    /// Owner tokens list (Address -> Vec<u32>)
    OwnerTokens(Address),
    /// List of all token IDs (Vec<u32>)
    TokenIds,
    /// Authorized commitment_core contract address (for settlement)
    CoreContract,
    /// Authorized minter addresses
    AuthorizedMinter(Address),
    /// Active status (token_id -> bool)
    ActiveStatus(u32),
    /// Reentrancy guard flag
    ReentrancyGuard,
}

#[cfg(test)]
mod tests;

// ============================================================================
// Contract Implementation
// ============================================================================

#[contract]
pub struct CommitmentNFTContract;

#[contractimpl]
impl CommitmentNFTContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initialize the NFT contract with an admin address
    pub fn initialize(e: Env, admin: Address) -> Result<(), ContractError> {
        if e.storage()
            .instance()
            .has(&access_control::AccessControlKey::Admin)
        {
            return Err(ContractError::AlreadyInitialized);
        }
        AccessControl::init_admin(&e, admin.clone())
            .map_err(|_| ContractError::AlreadyInitialized)?;
        e.storage().instance().set(&DataKey::Admin, &admin);
        e.storage().instance().set(&DataKey::TokenCounter, &0u32);
        Ok(())
    }

    /// Add an authorized contract to the whitelist (admin only)
    pub fn add_authorized_contract(
        e: Env,
        caller: Address,
        contract_address: Address,
    ) -> Result<(), ContractError> {
        AccessControl::add_authorized_contract(&e, caller, contract_address)
            .map_err(ContractError::from)
    }

    // ========================================================================
    // Access Control
    // ========================================================================

    /// Add an authorized minter (admin or commitment_core contract)
    pub fn add_authorized_minter(
        e: Env,
        caller: Address,
        minter: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;

        if caller != admin {
            return Err(ContractError::NotAuthorized);
        }

        e.storage()
            .instance()
            .set(&DataKey::AuthorizedMinter(minter), &true);

        Ok(())
    }

    /// Remove an authorized contract from the whitelist (admin only)
    pub fn remove_authorized_contract(
        e: Env,
        caller: Address,
        contract_address: Address,
    ) -> Result<(), ContractError> {
        AccessControl::remove_authorized_contract(&e, caller, contract_address)
            .map_err(ContractError::from)
    }

    /// Check if a contract address is authorized
    pub fn is_authorized(e: Env, contract_address: Address) -> bool {
        AccessControl::is_authorized(&e, &contract_address)
    }

    /// Update admin (admin only)
    pub fn update_admin(e: Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        AccessControl::update_admin(&e, caller, new_admin).map_err(ContractError::from)
    }

    /// Get the current admin address
    pub fn get_admin(e: Env) -> Result<Address, ContractError> {
        AccessControl::get_admin(&e).map_err(ContractError::from)
    }

    /// Validate commitment type
    fn is_valid_commitment_type(e: &Env, commitment_type: &String) -> bool {
        let safe = String::from_str(e, "safe");
        let balanced = String::from_str(e, "balanced");
        let aggressive = String::from_str(e, "aggressive");
        *commitment_type == safe || *commitment_type == balanced || *commitment_type == aggressive
    }

    /// Set the authorized commitment_core contract address for settlement
    /// Only the admin can call this function
    pub fn set_core_contract(e: Env, core_contract: Address) -> Result<(), ContractError> {
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        e.storage()
            .instance()
            .set(&DataKey::CoreContract, &core_contract);

        // Emit event for access control change
        e.events()
            .publish((Symbol::new(&e, "CoreContractSet"),), (core_contract,));

        Ok(())
    }

    /// Get the authorized commitment_core contract address
    pub fn get_core_contract(e: Env) -> Result<Address, ContractError> {
        e.storage()
            .instance()
            .get(&DataKey::CoreContract)
            .ok_or(ContractError::NotInitialized)
    }

    // ========================================================================
    // NFT Minting
    // ========================================================================

    /// Mint a new Commitment NFT
    ///
    /// # Arguments
    /// * `caller` - The address calling the mint function (must be authorized)
    /// * `owner` - The address that will own the NFT
    /// * `commitment_id` - Unique identifier for the commitment
    /// * `duration_days` - Duration of the commitment in days
    /// * `max_loss_percent` - Maximum allowed loss percentage (0-100)
    /// * `commitment_type` - Type of commitment ("safe", "balanced", "aggressive")
    /// * `initial_amount` - Initial amount committed
    /// * `asset_address` - Address of the asset contract
    ///
    /// # Returns
    /// The token_id of the newly minted NFT
    #[allow(clippy::too_many_arguments)]
    pub fn mint(
        e: Env,
        caller: Address,
        owner: Address,
        commitment_id: String,
        duration_days: u32,
        max_loss_percent: u32,
        commitment_type: String,
        initial_amount: i128,
        asset_address: Address,
    ) -> Result<u32, Error> {
        // Access control: only authorized addresses (admin or whitelisted contracts) can mint
        AccessControl::require_authorized(&e, &caller)?;

        // Validate parameters
        if duration_days == 0 {
            return Err(Error::InvalidDuration);
        }
        if max_loss_percent > 100 {
            return Err(Error::InvalidMaxLoss);
        }
        if !Self::is_valid_commitment_type(&e, &commitment_type) {
            return Err(Error::InvalidCommitmentType);
        }
        if initial_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Validate parameters
        if duration_days == 0 {
            e.storage()
                .instance()
                .set(&DataKey::ReentrancyGuard, &false);
            return Err(ContractError::InvalidDuration);
        }
        if max_loss_percent > 100 {
            e.storage()
                .instance()
                .set(&DataKey::ReentrancyGuard, &false);
            return Err(ContractError::InvalidMaxLoss);
        }
        if !Self::is_valid_commitment_type(&e, &commitment_type) {
            e.storage()
                .instance()
                .set(&DataKey::ReentrancyGuard, &false);
            return Err(ContractError::InvalidCommitmentType);
        }
        if initial_amount <= 0 {
            e.storage()
                .instance()
                .set(&DataKey::ReentrancyGuard, &false);
            return Err(ContractError::InvalidAmount);
        }

        // EFFECTS: Update state
        // Generate unique token_id
        let token_id: u32 = e
            .storage()
            .instance()
            .get(&DataKey::TokenCounter)
            .unwrap_or(0)
            + 1;
        e.storage()
            .instance()
            .set(&DataKey::TokenCounter, &token_id);

        // Calculate timestamps
        let created_at = e.ledger().timestamp();
        let seconds_per_day: u64 = 86400;
        let expires_at = created_at + (duration_days as u64 * seconds_per_day);

        // Create CommitmentMetadata
        let metadata = CommitmentMetadata {
            commitment_id: commitment_id.clone(),
            duration_days,
            max_loss_percent,
            commitment_type,
            created_at,
            expires_at,
            initial_amount,
            asset_address,
        };

        // Create CommitmentNFT
        let nft = CommitmentNFT {
            owner: owner.clone(),
            token_id,
            metadata,
            is_active: true,
            early_exit_penalty: 0,
        };

        // Store NFT data
        e.storage().instance().set(&DataKey::NFT(token_id), &nft);

        // Update owner balance
        let current_balance: u32 = e
            .storage()
            .instance()
            .get(&DataKey::OwnerBalance(owner.clone()))
            .unwrap_or(0);
        e.storage().instance().set(
            &DataKey::OwnerBalance(owner.clone()),
            &(current_balance + 1),
        );

        // Update owner tokens list
        let mut owner_tokens: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::OwnerTokens(owner.clone()))
            .unwrap_or(Vec::new(&e));
        owner_tokens.push_back(token_id);
        e.storage()
            .instance()
            .set(&DataKey::OwnerTokens(owner.clone()), &owner_tokens);

        // Add token_id to the list of all tokens
        let mut token_ids: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::TokenIds)
            .unwrap_or(Vec::new(&e));
        token_ids.push_back(token_id);
        e.storage().instance().set(&DataKey::TokenIds, &token_ids);

        // Clear reentrancy guard
        e.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &false);

        // Emit mint event
        e.events().publish(
            (symbol_short!("Mint"), token_id, owner.clone()),
            (commitment_id, e.ledger().timestamp()),
        );

        Ok(token_id)
    }

    // ========================================================================
    // NFT Query Functions
    // ========================================================================

    /// Get NFT metadata by token_id
    pub fn get_metadata(e: Env, token_id: u32) -> Result<CommitmentNFT, ContractError> {
        e.storage()
            .instance()
            .get(&DataKey::NFT(token_id))
            .ok_or(ContractError::TokenNotFound)
    }

    /// Get owner of NFT
    pub fn owner_of(e: Env, token_id: u32) -> Result<Address, ContractError> {
        let nft: CommitmentNFT = e
            .storage()
            .instance()
            .get(&DataKey::NFT(token_id))
            .ok_or(ContractError::TokenNotFound)?;

        Ok(nft.owner)
    }

    /// Check if NFT is active
    pub fn is_active(e: Env, token_id: u32) -> Result<bool, ContractError> {
        let nft: CommitmentNFT = e
            .storage()
            .instance()
            .get(&DataKey::NFT(token_id))
            .ok_or(ContractError::TokenNotFound)?;

        Ok(nft.is_active)
    }

    /// Get total supply of NFTs
    pub fn total_supply(e: Env) -> u32 {
        e.storage()
            .instance()
            .get(&DataKey::TokenCounter)
            .unwrap_or(0)
    }

    /// Get balance (number of NFTs) for an owner
    pub fn balance_of(e: Env, owner: Address) -> u32 {
        e.storage()
            .instance()
            .get(&DataKey::OwnerBalance(owner))
            .unwrap_or(0)
    }

    /// Get all NFT metadata
    pub fn get_all_metadata(e: Env) -> Vec<CommitmentNFT> {
        let token_ids: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::TokenIds)
            .unwrap_or(Vec::new(&e));

        let mut result = Vec::new(&e);
        for token_id in token_ids.iter() {
            if let Some(nft) = e.storage().instance().get(&DataKey::NFT(token_id)) {
                result.push_back(nft);
            }
        }
        result
    }

    /// Get all NFTs owned by an address
    pub fn get_nfts_by_owner(e: Env, owner: Address) -> Vec<CommitmentNFT> {
        let owner_tokens: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::OwnerTokens(owner))
            .unwrap_or(Vec::new(&e));

        let mut result = Vec::new(&e);
        for token_id in owner_tokens.iter() {
            if let Some(nft) = e.storage().instance().get(&DataKey::NFT(token_id)) {
                result.push_back(nft);
            }
        }
        result
    }

    // ========================================================================
    // NFT Transfer
    // ========================================================================

    /// Transfer NFT to new owner
    pub fn transfer(e: Env, from: Address, to: Address, token_id: u32) -> Result<(), Error> {
        // Require authorization from the sender
        from.require_auth();

        // CHECKS: Verify ownership
        let mut nft: CommitmentNFT = e
            .storage()
            .instance()
            .get(&DataKey::NFT(token_id))
            .ok_or(Error::TokenNotFound)?;

        // Verify ownership
        if nft.owner != from {
            return Err(Error::NotOwner);
        }

        // Check if NFT is still active (active NFTs may have transfer restrictions)
        // For now, we allow transfers regardless of active status
        // Uncomment below to restrict transfers of active NFTs:
        // if nft.is_active {
        //     return Err(Error::TransferNotAllowed);
        // }

        // EFFECTS: Update state
        // Update owner
        nft.owner = to.clone();
        e.storage().instance().set(&DataKey::NFT(token_id), &nft);

        // Update from balance
        let from_balance: u32 = e
            .storage()
            .instance()
            .get(&DataKey::OwnerBalance(from.clone()))
            .unwrap_or(0);
        if from_balance > 0 {
            e.storage()
                .instance()
                .set(&DataKey::OwnerBalance(from.clone()), &(from_balance - 1));
        }

        // Update to balance
        let to_balance: u32 = e
            .storage()
            .instance()
            .get(&DataKey::OwnerBalance(to.clone()))
            .unwrap_or(0);
        e.storage()
            .instance()
            .set(&DataKey::OwnerBalance(to.clone()), &(to_balance + 1));

        // Update from tokens list
        let mut from_tokens: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::OwnerTokens(from.clone()))
            .unwrap_or(Vec::new(&e));
        let mut new_from_tokens = Vec::new(&e);
        for t in from_tokens.iter() {
            if t != token_id {
                new_from_tokens.push_back(t);
            }
        }
        e.storage()
            .instance()
            .set(&DataKey::OwnerTokens(from.clone()), &new_from_tokens);

        // Update to tokens list
        let mut to_tokens: Vec<u32> = e
            .storage()
            .instance()
            .get(&DataKey::OwnerTokens(to.clone()))
            .unwrap_or(Vec::new(&e));
        to_tokens.push_back(token_id);
        e.storage()
            .instance()
            .set(&DataKey::OwnerTokens(to.clone()), &to_tokens);

        // Clear reentrancy guard
        e.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &false);

        // Emit transfer event
        e.events()
            .publish((symbol_short!("transfer"), from, to), token_id);

        Ok(())
    }

    // ========================================================================
    // Settlement
    // ========================================================================

    /// Mark NFT as settled (after maturity)
    pub fn settle(e: Env, token_id: u32) -> Result<(), Error> {
        // Get the NFT
        let mut nft: CommitmentNFT = e
            .storage()
            .instance()
            .get(&DataKey::NFT(token_id))
            .ok_or(Error::TokenNotFound)?;

        // Check if already settled
        if !nft.is_active {
            return Err(Error::AlreadySettled);
        }

        // Verify the commitment has expired
        let current_time = e.ledger().timestamp();
        if current_time < nft.metadata.expires_at {
            return Err(Error::NotExpired);
        }

        // EFFECTS: Update state
        // Mark as inactive (settled)
        nft.is_active = false;
        e.storage().instance().set(&DataKey::NFT(token_id), &nft);

        // Clear reentrancy guard
        e.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &false);

        // Emit settle event
        e.events()
            .publish((symbol_short!("Settle"), token_id), e.ledger().timestamp());

        Ok(())
    }

    /// Check if an NFT has expired (based on time)
    pub fn is_expired(e: Env, token_id: u32) -> Result<bool, ContractError> {
        let nft: CommitmentNFT = e
            .storage()
            .instance()
            .get(&DataKey::NFT(token_id))
            .ok_or(ContractError::TokenNotFound)?;

        let current_time = e.ledger().timestamp();
        Ok(current_time >= nft.metadata.expires_at)
    }

    /// Check if a token exists
    pub fn token_exists(e: Env, token_id: u32) -> bool {
        e.storage().instance().has(&DataKey::NFT(token_id))
    }

    /// Set emergency mode (admin only)
    pub fn set_emergency_mode(e: Env, caller: Address, enabled: bool) -> Result<(), ContractError> {
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)?;
        admin.require_auth();

        if caller != admin {
            return Err(ContractError::NotAuthorized);
        }

        EmergencyControl::set_emergency_mode(&e, enabled);
        Ok(())
    }
}

#[cfg(all(test, feature = "benchmark"))]
mod benchmarks;

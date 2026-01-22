#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, Map, i128, symbol_short, Symbol};
// Storage keys
#[contracttype]
pub enum DataKey {
    Admin,
    TotalSupply,
    Nft(u32),   // token_id -> CommitmentNFT
    Owner(u32), // token_id -> Address
    AuthorizedMinter(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidDuration = 4,
    InvalidMaxLoss = 5,
    InvalidCommitmentType = 6,
    InvalidAmount = 7,
    TokenNotFound = 8,
}

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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentNFT {
    pub owner: Address,
    pub token_id: u32,
    pub metadata: CommitmentMetadata,
    pub is_active: bool,
    pub early_exit_penalty: u32,
}

// Storage keys for access control
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
const AUTHORIZED_KEY: Symbol = symbol_short!("AUTH");
const INITIALIZED_KEY: Symbol = symbol_short!("INIT");
const MINT: soroban_sdk::Symbol = symbol_short!("mint");

// Events
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminChangedEvent {
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedContractAddedEvent {
    pub contract_address: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedContractRemovedEvent {
    pub contract_address: Address,
}

#[contract]
pub struct CommitmentNFTContract;

// Access control helper functions
impl CommitmentNFTContract {
    /// Get the admin address from storage
    fn get_admin(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("Contract not initialized")
    }

    /// Set the admin address in storage
    fn set_admin(e: &Env, admin: &Address) {
        e.storage().instance().set(&ADMIN_KEY, admin);
    }

    /// Check if contract is initialized
    fn is_initialized(e: &Env) -> bool {
        e.storage().instance().has(&INITIALIZED_KEY)
    }

    /// Mark contract as initialized
    fn set_initialized(e: &Env) {
        e.storage().instance().set(&INITIALIZED_KEY, &true);
    }

    /// Check if caller is admin
    fn require_admin(e: &Env) {
        let admin = Self::get_admin(e);
        let caller = e.invoker();
        if caller != admin {
            panic!("Unauthorized: admin access required");
        }
    }

    /// Check if an address is authorized (admin or whitelisted contract)
    fn is_authorized(e: &Env, address: &Address) -> bool {
        let admin = Self::get_admin(e);
        if *address == admin {
            return true;
        }
        
        // Check whitelist
        let key = (AUTHORIZED_KEY, address);
        e.storage().instance().has(&key)
    }

    /// Require that caller is authorized (admin or whitelisted)
    fn require_authorized(e: &Env) {
        let caller = e.invoker();
        if !Self::is_authorized(e, &caller) {
            panic!("Unauthorized: admin or authorized contract access required");
        }
    }

    /// Add an authorized contract to whitelist
    fn add_authorized_contract(e: &Env, contract_address: &Address) {
        let key = (AUTHORIZED_KEY, contract_address);
        e.storage().instance().set(&key, &true);
        
        // Emit event
        e.events().publish(
            (symbol_short!("auth_add"), contract_address),
            AuthorizedContractAddedEvent {
                contract_address: contract_address.clone(),
            },
        );
    }

    /// Remove an authorized contract from whitelist
    fn remove_authorized_contract(e: &Env, contract_address: &Address) {
        let key = (AUTHORIZED_KEY, contract_address);
        if e.storage().instance().has(&key) {
            e.storage().instance().remove(&key);
            
            // Emit event
            e.events().publish(
                (symbol_short!("auth_rm"), contract_address),
                AuthorizedContractRemovedEvent {
                    contract_address: contract_address.clone(),
                },
            );
        }
    }
}

#[contractimpl]
impl CommitmentNFTContract {
  
  ///start
    /// Initialize the NFT contract
    pub fn initialize(e: Env, admin: Address) {
        if Self::is_initialized(&e) {
            panic!("Contract already initialized");
        }
        
        Self::set_admin(&e, &admin);
        Self::set_initialized(&e);
    }

    /// Transfer admin role to a new address (admin-only)
    pub fn transfer_admin(e: Env, new_admin: Address) {
        Self::require_admin(&e);
        
        let old_admin = Self::get_admin(&e);
        Self::set_admin(&e, &new_admin);
        
        // Emit event
        e.events().publish(
            symbol_short!("admin_chg"),
            AdminChangedEvent {
                old_admin,
                new_admin: new_admin.clone(),
            },
        );
    }

    /// Get the current admin address
    pub fn get_admin(e: Env) -> Address {
        Self::get_admin(&e)
    }

    /// Add an authorized contract to whitelist (admin-only)
    pub fn add_authorized_contract(e: Env, contract_address: Address) {
        Self::require_admin(&e);
        Self::add_authorized_contract(&e, &contract_address);
    }

    /// Remove an authorized contract from whitelist (admin-only)
    pub fn remove_authorized_contract(e: Env, contract_address: Address) {
        Self::require_admin(&e);
        Self::remove_authorized_contract(&e, &contract_address);
    pub fn initialize(e: Env, admin: Address) -> Result<(), Error> {
        if e.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        e.storage().instance().set(&DataKey::Admin, &admin);
        e.storage().instance().set(&DataKey::TotalSupply, &0u32);
        Ok(())
    }

    /// Add an authorized minter (admin or commitment_core contract)
    pub fn add_authorized_minter(e: Env, caller: Address, minter: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        e.storage()
            .instance()
            .set(&DataKey::AuthorizedMinter(minter), &true);
        Ok(())
    }

    /// Check if caller is authorized to mint
    fn is_authorized_minter(e: &Env, caller: &Address) -> bool {
        if let Some(admin) = e
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
        {
            if *caller == admin {
                return true;
            }
        }
        e.storage()
            .instance()
            .get(&DataKey::AuthorizedMinter(caller.clone()))
            .unwrap_or(false)
    }

    /// Validate commitment type
    fn is_valid_commitment_type(e: &Env, commitment_type: &String) -> bool {
        let safe = String::from_str(e, "safe");
        let balanced = String::from_str(e, "balanced");
        let aggressive = String::from_str(e, "aggressive");
        *commitment_type == safe || *commitment_type == balanced || *commitment_type == aggressive
    }
///end
    /// Check if an address is authorized
    pub fn is_authorized(e: Env, contract_address: Address) -> bool {
        Self::is_authorized(&e, &contract_address)
    }

    /// Mint a new Commitment NFT (admin-only)
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
        caller.require_auth();

        // Access control: only authorized addresses can mint
        if !Self::is_authorized_minter(&e, &caller) {
            return Err(Error::Unauthorized);
        }

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

        // Generate unique sequential token_id
        let total_supply: u32 = e
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .ok_or(Error::NotInitialized)?;
        let token_id = total_supply + 1;

        // Calculate timestamps
        let created_at = e.ledger().timestamp();
        let duration_seconds = (duration_days as u64) * 24 * 60 * 60;
        let expires_at = created_at + duration_seconds;

        // Create metadata
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

        // Create NFT
        let nft = CommitmentNFT {
            owner: owner.clone(),
            token_id,
            metadata,
            is_active: true,
            early_exit_penalty: 0,
        };

        // Store NFT and ownership
        e.storage().persistent().set(&DataKey::Nft(token_id), &nft);
        e.storage()
            .persistent()
            .set(&DataKey::Owner(token_id), &owner);

        // Increment total supply
        e.storage().instance().set(&DataKey::TotalSupply, &token_id);

        // Emit mint event
        e.events()
            .publish((MINT, token_id), (owner, commitment_id, created_at));

        Ok(token_id)
    }

    /// Get NFT metadata by token_id
    pub fn get_metadata(e: Env, token_id: u32) -> Result<CommitmentMetadata, Error> {
        let nft: CommitmentNFT = e
            .storage()
            .persistent()
            .get(&DataKey::Nft(token_id))
            .ok_or(Error::TokenNotFound)?;
        Ok(nft.metadata)
    }

    /// Get owner of NFT
    pub fn owner_of(e: Env, token_id: u32) -> Result<Address, Error> {
        e.storage()
            .persistent()
            .get(&DataKey::Owner(token_id))
            .ok_or(Error::TokenNotFound)
    }

    /// Transfer NFT to new owner
    pub fn transfer(e: Env, from: Address, _to: Address, _token_id: u32) {
        // Verify caller is the owner
        let caller = e.invoker();
        if caller != from {
            panic!("Unauthorized: only owner can transfer");
        }
        
        // TODO: Verify ownership
        // TODO: Check if transfer is allowed (not locked)
        // TODO: Update owner
        // TODO: Emit transfer event
    }

    /// Check if NFT is active
    pub fn is_active(e: Env, token_id: u32) -> Result<bool, Error> {
        let nft: CommitmentNFT = e
            .storage()
            .persistent()
            .get(&DataKey::Nft(token_id))
            .ok_or(Error::TokenNotFound)?;
        Ok(nft.is_active)
    }

    /// Get total supply
    pub fn total_supply(e: Env) -> Result<u32, Error> {
        e.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .ok_or(Error::NotInitialized)
    }

    /// Mark NFT as settled (after maturity) - authorized contracts only
    pub fn settle(e: Env, _token_id: u32) {
        Self::require_authorized(&e);
        
        // TODO: Verify expiration
        // TODO: Mark as inactive
        // TODO: Emit settle event
    }
}

#[cfg(test)]
mod tests;

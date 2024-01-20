use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use serde::{Serialize, Deserialize};
use std::io::Cursor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bincode::{serialize};
use erc20_macro::generate_abi;
use std::ffi::{CString, CStr};

#[derive(Serialize, Deserialize,Clone)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owner_of: Vec<String>,        // Vector to store owner addresses
    pub token_to_ipfs: Vec<String>,    // Vector to store IPFS hashes
    //pub owner_of: HashMap<u64, String>,  // tokenId -> owner address
    //pub token_to_ipfs: HashMap<u64, String>,  // tokenId -> IPFS hash
    pub token_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenDetails {
    pub owner: String,
    pub ipfs_link: String,
}

fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}

pub struct GlobalState {
    token_ptr: Option<*mut ERC721Token>,
    token_details_buffer: Vec<u8>, // Use a dynamic Vec<u8> for the buffer
}

static mut GLOBAL_STATE: GlobalState = GlobalState {
    token_ptr: None,
    token_details_buffer: Vec::new(), // Initialize with an empty Vec
};

#[generate_abi]
impl ERC721Token {
    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC721Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }
    #[no_mangle]
    pub fn to_memory(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let serialized_data = serialize(self)?;
        Ok(serialized_data)
    }
    #[no_mangle]
    pub extern "C" fn initialize(
        name_ptr: *mut u8,
        name_len: usize,
        symbol_ptr: *mut u8,
        symbol_len: usize,
    ) {
        // Extract name and symbol from wasm memory
        let name = extract_string_from_wasm_memory(name_ptr, name_len);
        let symbol = extract_string_from_wasm_memory(symbol_ptr, symbol_len);

        let token = ERC721Token {
            name: name,
            symbol: symbol,
            owner_of: Vec::new(),
            token_to_ipfs: Vec::new(),
            token_id: 0,
        };

        // Box and convert the token into a raw pointer
        let token_ptr = Box::into_raw(Box::new(token));
        unsafe {
            GLOBAL_STATE.token_ptr = Some(token_ptr);
        }
    }

    #[no_mangle]
    pub extern "C" fn mint(
        owner_ptr: *const u8,
        owner_len: usize,
        ipfs_hash_ptr: *const u8,
        ipfs_hash_len: usize,
    ) -> u32 {
        let token = match unsafe { GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &mut *ptr },
            None => {
                println!("Mint: Failed to mint, uninitialized TOKEN_PTR.");
                return u32::MAX; // Error value indicating uninitialized TOKEN_PTR.
            }
        };
        // Convert raw pointers to Rust strings using from_utf8_lossy
        let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
}
}
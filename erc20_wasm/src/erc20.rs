use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use erc20_macro::generate_abi;
use std::io::Cursor;
use serde::{Serialize, Deserialize};
use std::ffi::{CString, CStr};

#[derive(Serialize, Deserialize)]
pub struct ERC20Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub balances: HashMap<String, u64>,
    pub allowed: HashMap<String, HashMap<String, u64>>,
}

fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}

impl ERC20Token {
    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC20Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }
    fn serialize_to_memory(token: &ERC20Token, buffer: *mut u8) -> Result<usize, Box<dyn std::error::Error>> {
        let mut writer = unsafe {
            // Assuming the buffer is large enough to hold the serialized data
            Cursor::new(std::slice::from_raw_parts_mut(buffer, 1024)) // replace 1024 with suitable max size
        };
        serialize_into(&mut writer, &token)?;
        Ok(writer.position() as usize)
    }

    #[no_mangle]
    pub extern "C" fn initialize(
        name_ptr: *mut u8, 
        name_len: usize, 
        symbol_ptr: *mut u8, 
        symbol_len: usize,
        decimals: u8, 
        initial_supply: u64
    ) -> *mut ERC20Token {
        // Extract name and symbol from wasm memory
        let name = extract_string_from_wasm_memory(name_ptr, name_len);
        let symbol = extract_string_from_wasm_memory(symbol_ptr, symbol_len);
    
        let mut balances = HashMap::new();
        balances.insert("Contract_Owner".to_string(), initial_supply);
    
        let token = ERC20Token {
            name,
            symbol,
            decimals,
            total_supply: initial_supply,
            balances,
            allowed: HashMap::new(),
        };
    
        let token_ptr = Box::into_raw(Box::new(token));
    
        token_ptr
    }

    #[no_mangle]
    pub extern "C" fn store_token_in_memory(token_ptr: *mut ERC20Token, buffer: *mut u8) -> usize {
        let token = unsafe { &*token_ptr };
        let len = Self::serialize_to_memory(token, buffer).expect("Failed to serialize");
        len
    }

    #[no_mangle]
    pub extern "C" fn load_token_from_memory(buffer: *const u8, len: usize) -> *mut ERC20Token {
        let token = Self::deserialize_from_memory(buffer, len).expect("Failed to deserialize");
        Box::into_raw(Box::new(token))
    }

    #[no_mangle]
    pub extern "C" fn destroy(token_ptr: *mut ERC20Token) {
        // Deallocate the memory when you're done with the ERC20Token instance
        unsafe {
            Box::from_raw(token_ptr);
        }
    }

    #[no_mangle]
    pub extern "C" fn balance_of(&self, owner: *const i8) -> u64 {
        let owner_str = unsafe { CStr::from_ptr(owner) }.to_str().unwrap();
        *self.balances.get(owner_str).unwrap_or(&0)
    }

    #[no_mangle]
    pub extern "C" fn transfer(&mut self, from: *const i8, to: *const i8, value: u64) -> i32 {
        let from_str = unsafe { CStr::from_ptr(from) }.to_str().unwrap().to_string();
        let to_str = unsafe { CStr::from_ptr(to) }.to_str().unwrap().to_string();

        let sender_balance = self.balance_of(from);
        if sender_balance < value {
            return -1; // Insufficient balance
        }

        let receiver_balance = self.balance_of(to);
        self.balances.insert(from_str.clone(), sender_balance - value);
        self.balances.insert(to_str.clone(), receiver_balance + value);

        0 // Success
    }

    #[no_mangle]
    pub extern "C" fn approve(&mut self, owner: *const i8, spender: *const i8, value: u64) -> i32 {
        let owner_str = unsafe { CStr::from_ptr(owner) }.to_str().unwrap().to_string();
        let spender_str = unsafe { CStr::from_ptr(spender) }.to_str().unwrap().to_string();

        let allowances = self.allowed.entry(owner_str).or_insert(HashMap::new());
        allowances.insert(spender_str, value);

        0 // Success
    }

    #[no_mangle]
    pub extern "C" fn allowance(&self, owner: *const i8, spender: *const i8) -> u64 {
        let owner_str = unsafe { CStr::from_ptr(owner) }.to_str().unwrap().to_string();
        let spender_str = unsafe { CStr::from_ptr(spender) }.to_str().unwrap().to_string();

        if let Some(allowances) = self.allowed.get(&owner_str) {
            *allowances.get(&spender_str).unwrap_or(&0)
        } else {
            0
        }
    }

    #[no_mangle]
    pub extern "C" fn transfer_from(&mut self, spender: *const i8, from: *const i8, to: *const i8, value: u64) -> i32 {
        let spender_str = unsafe { CStr::from_ptr(spender) }.to_str().unwrap().to_string();
        let from_str = unsafe { CStr::from_ptr(from) }.to_str().unwrap().to_string();
        let to_str = unsafe { CStr::from_ptr(to) }.to_str().unwrap().to_string();

        let allowance = self.allowance(from, spender);
        if allowance < value {
            return -1; // Allowance exceeded
        }

        if self.transfer(from, to, value) == -1 {
            return -1; // Transfer failed
        }

        let allowances = self.allowed.get_mut(&from_str).unwrap();
        allowances.insert(spender_str, allowance - value);

        0 // Success
    }

    #[no_mangle]
    pub extern "C" fn read_name(&self, buffer: *mut u8, buffer_len: usize) -> isize {
        self.string_to_buffer(&self.name, buffer, buffer_len)
    }

    #[no_mangle]
    pub extern "C" fn read_symbol(&self, buffer: *mut u8, buffer_len: usize) -> isize {
        self.string_to_buffer(&self.symbol, buffer, buffer_len)
    }

    fn string_to_buffer(&self, source: &str, buffer: *mut u8, buffer_len: usize) -> isize {
        let bytes = source.as_bytes();
        if bytes.len() > buffer_len {
            return -1;  // Buffer too small
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
        }
        bytes.len() as isize
    }

    #[no_mangle]
    pub extern "C" fn total_supply(&self) -> u64 {
        self.total_supply
    }

    #[no_mangle]
    pub extern "C" fn read_decimals(&self) -> u8 {
        self.decimals
    }
}

SUMOTEX MAINNET
== OFFICIAL COMPILER ====
COPYRIGHT AND OWN BY SUMOTEX HOLDING

//FOR WASM_GENERATOR_RUST
//HOW TO RUN IT
-- erc20
-- erc20_macro
---- Generate ABI
------RUN THE FOLLOWING: cargo new --lib generate_abi_macro
-- erc20_wasm
---- Generation of WASM file here.
-- add_derive_macro
1. Change the target towards the file
2. Go to erc20_wasm and edit the cargo.toml towards the right target
3. Check if #[generate_abi] exist and run cargo build --target wasm32-wasi --release
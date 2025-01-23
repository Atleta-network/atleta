dev:
	cargo build --release --features devnet-runtime

testnet:
	cargo build --release --features testnet-runtime

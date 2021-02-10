SECRETCLI = docker exec -it secretdev /usr/bin/secretcli

.PHONY: all
all: clippy test

.PHONY: check
check:
	cargo check

.PHONY: check-receiver
check-receiver:
	$(MAKE) -C tests/example-receiver check

.PHONY: clippy
clippy:
	cargo clippy

.PHONY: clippy-receiver
clippy-receiver:
	$(MAKE) -C tests/example-receiver clippy

.PHONY: test
test: unit-test unit-test-receiver integration-test

.PHONY: unit-test
unit-test:
	cargo test

.PHONY: unit-test-receiver
unit-test-receiver:
	$(MAKE) -C tests/example-receiver unit-test

.PHONY: integration-test
integration-test: compile-optimized compile-optimized-receiver
	tests/integration.sh

compile-optimized-receiver:
	$(MAKE) -C tests/example-receiver compile-optimized

.PHONY: list-code
list-code:
	$(SECRETCLI) query compute list-code

.PHONY: compile _compile
compile: _compile contract.wasm.gz
_compile:
	cargo build --target wasm32-unknown-unknown --locked
	cp ./target/wasm32-unknown-unknown/debug/*.wasm ./contract.wasm

.PHONY: compile-optimized _compile-optimized
compile-optimized: _compile-optimized
_compile-optimized:
	RUSTFLAGS='-C link-arg=-s' cargo +nightly build --release --target wasm32-unknown-unknown --locked
	@# The following line is not necessary, may work only on linux (extra size optimization)
	# wasm-opt -Os ./target/wasm32-unknown-unknown/release/*.wasm -o .
	cp ./target/wasm32-unknown-unknown/release/*.wasm ./build/

.PHONY: compile-w-debug-print _compile-w-debug-print
compile-w-debug-print: _compile-w-debug-print
_compile-w-debug-print:
	RUSTFLAGS='-C link-arg=-s' cargo +nightly build --release --target wasm32-unknown-unknown --locked
	cd contracts/secretswap_pair && RUSTFLAGS='-C link-arg=-s' cargo build --release --features debug-print --target wasm32-unknown-unknown --locked
	cp ./target/wasm32-unknown-unknown/release/*.wasm ./build/

.PHONY: compile-optimized-reproducible
compile-optimized-reproducible:
	docker run --rm -v "$$(pwd)":/contract \
		--mount type=volume,source="$$(basename "$$(pwd)")_cache",target=/code/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		enigmampc/secret-contract-optimizer:1.0.4

# After make start-server is streaming blocks, this will setup the AMM and send you some SCRT and ETH
# change the secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9 to your address
# scripts/setup.sh &&
#	 docker exec -it secretdev secretcli tx send a secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9 100000000uscrt -y -b block &&
#	 docker exec -it secretdev secretcli tx compute execute secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg '{"transfer":{"recipient":"secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9","amount":"100000000"}}' --from a -y -b block
.PHONY: start-server
start-server: # CTRL+C to stop
	docker run -it --rm \
		-p 26657:26657 -p 26656:26656 -p 1337:1337 \
		-v $$(pwd):/root/code \
		--name secretdev enigmampc/secret-network-sw-dev:latest

.PHONY: schema
schema:
	cargo run --example schema

.PHONY: clean
clean:
	cargo clean
	rm -f ./contract.wasm ./contract.wasm.gz
	$(MAKE) -C tests/example-receiver clean


# token: 1
# factory: 2
# pair: 3
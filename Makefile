# Rebuild native/sqlite.wasm from native/. Needs the wasm32-wasip2 Rust target
# (`rustup target add wasm32-wasip2`) and wasi-sdk for SQLite's C
# (https://github.com/WebAssembly/wasi-sdk/releases); WASI_SDK names where it is.
WASI_SDK ?= $(HOME)/.local/share/wasi-sdk-25.0-x86_64-linux

wasm:
	cd native && \
	CC_wasm32_wasip2="$(WASI_SDK)/bin/clang" \
	AR_wasm32_wasip2="$(WASI_SDK)/bin/llvm-ar" \
	CFLAGS_wasm32_wasip2="--sysroot=$(WASI_SDK)/share/wasi-sysroot" \
	LIBSQLITE3_FLAGS="-DSQLITE_THREADSAFE=0 -DSQLITE_OMIT_LOAD_EXTENSION -DSQLITE_OMIT_WAL" \
	cargo build --release --target wasm32-wasip2
	cp native/target/wasm32-wasip2/release/brood_sqlite.wasm native/sqlite.wasm

.PHONY: wasm

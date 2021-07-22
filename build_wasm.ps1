# Build wasm binary
cargo build --lib --target wasm32-unknown-unknown --release

# Reduce binary size
#wasm-gc "./target/wasm32-unknown-unknown/release/customasm.wasm" -o "./target/wasm32-unknown-unknown/release/customasm.gc.wasm"

# Copy to web folder
Copy-Item -Path "./target/wasm32-unknown-unknown/release/customasm.wasm" -Destination "./web/customasm.gc.wasm"


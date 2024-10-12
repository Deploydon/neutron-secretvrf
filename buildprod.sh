cargo build
cargo schema
cargo wasm
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  		--platform linux/amd64 \
  cosmwasm/workspace-optimizer:0.16.0


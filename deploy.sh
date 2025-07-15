#!/bin/bash

log() {
    echo -e "\n\033[1;32m[LOG] $1\033[0m"
}

log "Checking Rust target..."
if ! rustup target list | grep -q "wasm32v1-none (installed)"; then
    log "Adding wasm32v1-none target..."
    rustup target add wasm32v1-none
    if [ $? -ne 0 ]; then
        error "Failed to add wasm32v1-none target"
        exit 1
    fi
fi

# Build the contracts
log "Building contracts..."
cargo build --target wasm32v1-none --release
if [ $? -ne 0 ]; then
    echo -e "\033[1;31m[ERROR] Build failed\033[0m"
    exit 1
fi

# PoolFactory deployment
log "Uploading PoolFactory contract..."
pool_factory_upload_output=$(stellar contract upload \
  --network testnet \
  --source kennyv2 \
  --wasm target/wasm32v1-none/release/PoolFactory.wasm)
echo "$pool_factory_upload_output"

log "Deploying PoolFactory contract with wasm hash: $pool_factory_upload_output"
pool_factory_deploy_output=$(stellar contract deploy \
  --wasm-hash $pool_factory_upload_output \
  --source kennyv2 \
  --network testnet \
  --alias PoolFactory \
  -- --admin GC5QOPGD536F3M5PASQCMZUG7IGA2ARFE4KB46FAOKXZKHGOUPTMXP7W)
echo "[LOG] PoolFactory contract ID: $pool_factory_deploy_output"

# Extract contract ID from output (remove any extra text)
pool_factory_contract_id=$(echo "$pool_factory_deploy_output" | tr -d '\n' | sed 's/.*Contract ID: \([A-Z0-9]*\).*/\1/')

# TokenLauncher deployment
log "Uploading TokenLauncher contract..."
token_launcher_upload_output=$(stellar contract upload \
  --network testnet \
  --source kennyv2 \
  --wasm target/wasm32v1-none/release/tokenfactory.wasm)

log "Deploying TokenLauncher contract with wasm hash: $token_launcher_upload_output"
token_launcher_deploy_output=$(stellar contract deploy \
  --wasm-hash $token_launcher_upload_output \
  --source kennyv2 \
  --network testnet \
  --alias TokenLauncher \
  -- --admin GC5QOPGD536F3M5PASQCMZUG7IGA2ARFE4KB46FAOKXZKHGOUPTMXP7W)
echo "[LOG] TokenLauncher contract ID: $token_launcher_deploy_output"

# Extract contract ID from output
token_launcher_contract_id=$(echo "$token_launcher_deploy_output" | tr -d '\n' | sed 's/.*Contract ID: \([A-Z0-9]*\).*/\1/')

# USDTToken deployment
log "Uploading USDTToken contract..."
usdt_token_upload_output=$(stellar contract upload \
  --network testnet \
  --source kennyv2 \
  --wasm target/wasm32v1-none/release/token.wasm)

log "Deploying USDTToken contract with wasm hash: $usdt_token_upload_output"
usdt_token_deploy_output=$(stellar contract deploy \
  --wasm-hash $usdt_token_upload_output \
  --source kennyv2 \
  --network testnet \
  --alias USDTToken \
  -- --admin GC5QOPGD536F3M5PASQCMZUG7IGA2ARFE4KB46FAOKXZKHGOUPTMXP7W --decimal 6 --name "Tether USD" --symbol USDT)
echo "[LOG] USDTToken contract ID: $usdt_token_deploy_output"

# Extract contract ID from output
usdt_token_contract_id=$(echo "$usdt_token_deploy_output" | tr -d '\n' | sed 's/.*Contract ID: \([A-Z0-9]*\).*/\1/')

# Upload additional contracts (pool and tokenlaunch)
log "Uploading Pool contract..."
pool_upload_output=$(stellar contract upload \
  --network testnet \
  --source kennyv2 \
  --wasm target/wasm32v1-none/release/pool.wasm)
echo "$pool_upload_output"
echo "[LOG] Pool wasm hash: $pool_upload_output"


log "Deploying sample Pool contract..."
sample_pool_deploy_output=$(stellar contract deploy \
  --wasm-hash $pool_upload_output \
  --source kennyv2 \
  --network testnet \
  --alias Pool \
  -- --token_a CCVQ4H65EXQTPONOYK7CTH6JMCAWKJ4RP257FE2MA2UCF2AHVRHGQNTA --token_b CDIJAM6NYMJG5BCATG4TY75GCO4YP4ZYQHTFMH6KH64GEELIM7XH7E4E --lp_token_name "Cosmo LP Token" --lp_token_symbol "COSMO")
echo "[LOG] Sample Pool contract ID: $sample_pool_deploy_output"
sample_pool_contract_id=$(echo "$sample_pool_deploy_output" | tr -d '\n' | sed 's/.*Contract ID: \([A-Z0-9]*\).*/\1/')


log "Uploading TokenLaunch contract..."
meme_token_upload_output=$(stellar contract upload \
  --network testnet \
  --source kennyv2 \
  --wasm target/wasm32v1-none/release/tokenlaunch.wasm)
echo "$meme_token_upload_output"
echo "[LOG] Token wasm hash: $meme_token_upload_output"


log "Copying .stellar directory to UI..."
cp -R .stellar/* ../cosmoUI/.stellar/

# Generate TypeScript bindings
log "Generating TypeScript bindings..."
stellar contract bindings typescript \
  --network testnet \
  --contract-id $pool_factory_contract_id \
  --output-dir ../cosmoUI/packages/PoolFactory --overwrite

stellar contract bindings typescript \
  --network testnet \
  --contract-id $token_launcher_contract_id \
  --output-dir ../cosmoUI/packages/TokenLauncher --overwrite

stellar contract bindings typescript \
  --network testnet \
  --contract-id $usdt_token_contract_id \
  --output-dir ../cosmoUI/packages/USDTToken --overwrite

stellar contract bindings typescript \
  --network testnet \
  --contract-id $sample_pool_contract_id \
  --output-dir ../cosmoUI/packages/Pool --overwrite

# Create deployment.ts file
log "Creating deployment.ts file..."
DEPLOYMENT_TS_PATH="../cosmoUI/packages/deployment.ts"

cat > $DEPLOYMENT_TS_PATH << EOF
// Auto-generated by deployment script
export const CONTRACT_ADDRESSES = {
    PoolFactory: "$pool_factory_contract_id",
    TokenLauncher: "$token_launcher_contract_id",
    USDTToken: "$usdt_token_contract_id",
    PoolWasmHash: "$pool_upload_output",
    MemeTokenWasmHash: "$meme_token_upload_output"
};

export const ADMIN_ADDRESS = "GC5QOPGD536F3M5PASQCMZUG7IGA2ARFE4KB46FAOKXZKHGOUPTMXP7W";
EOF
# Build the generated TypeScript packages
log "Building TypeScript packages..."

log "Building PoolFactory package..."
cd ../cosmoUI/packages/PoolFactory
npm install --force
npm run build

log "Building TokenLauncher package..."
cd ../TokenLauncher
npm install --force
npm run build

log "Building USDTToken package..."
cd ../USDTToken
npm install --force
npm run build

log "Building Pool package..."
cd ../Pool
npm install --force
npm run build



log "All packages built successfully!"


log "Deployment completed successfully!"
log "PoolFactory Contract ID: $pool_factory_contract_id"
log "TokenLauncher Contract ID: $token_launcher_contract_id"
log "USDTToken Contract ID: $usdt_token_contract_id"
log "Pool Wasm Hash: $pool_upload_output"
log "MemeToken Wasm Hash: $meme_token_upload_output"




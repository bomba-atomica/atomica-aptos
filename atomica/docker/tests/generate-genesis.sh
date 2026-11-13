#!/bin/bash
# Generate Aptos genesis for Docker testnet
set -e

# Enable debug mode if ATOMICA_DEBUG_TESTNET is set
if [ -n "$ATOMICA_DEBUG_TESTNET" ]; then
    set -x
fi

# Debug logging helper
debug() {
    if [ -n "$ATOMICA_DEBUG_TESTNET" ]; then
        echo "[DEBUG $(date -Iseconds)] $*" >&2
    fi
}

# Redirect verbose aptos CLI output unless debugging
if [ -z "$ATOMICA_DEBUG_TESTNET" ]; then
    APTOS_OUTPUT="/dev/null"
else
    APTOS_OUTPUT="/dev/stdout"
fi

NUM_VALIDATORS="${1:-4}"
CHAIN_ID="${2:-4}"
BASE_IP="${3:-172.19.0.10}"
STAKE_AMOUNT="100000000000000"  # 1M APT in octas
# Set TIMELOCK_DKG_ENABLED=true to turn the timelock DKG on at genesis (layout flag
# timelock_dkg_enabled). It also needs the chunky DKG and validator transactions on.
TIMELOCK_DKG_ENABLED="${TIMELOCK_DKG_ENABLED:-false}"
case "$TIMELOCK_DKG_ENABLED" in
    true|false) ;;
    *) echo "TIMELOCK_DKG_ENABLED must be true or false, got '$TIMELOCK_DKG_ENABLED'" >&2; exit 1 ;;
esac

# With the timelock DKG on, genesis pins the SHA3-256 of the shipped timelock parameters blob (layout field
# timelock_public_parameters_hash). Off chain id 4 genesis fails without it. The validator image copies
# atomica/timelock-params/timelock-public-parameters.sha3-256 to the default path below.
TIMELOCK_PARAMETERS_HASH_FILE="${TIMELOCK_PARAMETERS_HASH_FILE:-/opt/aptos/timelock-params/timelock-public-parameters.sha3-256}"
TIMELOCK_HASH_LINE=""
if [ "$TIMELOCK_DKG_ENABLED" = "true" ]; then
    if [ ! -f "$TIMELOCK_PARAMETERS_HASH_FILE" ]; then
        echo "TIMELOCK_DKG_ENABLED=true needs the timelock parameters hash at $TIMELOCK_PARAMETERS_HASH_FILE" >&2
        exit 1
    fi
    TIMELOCK_HASH_LINE="timelock_public_parameters_hash: \"$(tr -d '[:space:]' < "$TIMELOCK_PARAMETERS_HASH_FILE")\""
fi

# Set ETH_CHAIN_ID to an EIP-155 chain id to turn Ethereum header attestation on at genesis (layout
# field eth_chain_id). Validators also need eth_header_observer enabled in their node config.
ETH_CHAIN_ID="${ETH_CHAIN_ID:-}"
case "$ETH_CHAIN_ID" in
    ""|*[!0-9]*) [ -z "$ETH_CHAIN_ID" ] || { echo "ETH_CHAIN_ID must be a decimal chain id, got '$ETH_CHAIN_ID'" >&2; exit 1; } ;;
esac
# With ETH_CHAIN_ID set, every validator observes ETH_RPC_URL (required) at ETH_BLOCK_TAG (default
# finalized; an anvil devnet without finality uses latest).
ETH_CHAIN_ID_LINE=""
ETH_OBSERVER_CONFIG=""
if [ -n "$ETH_CHAIN_ID" ]; then
    if [ -z "${ETH_RPC_URL:-}" ]; then
        echo "ETH_RPC_URL is required when ETH_CHAIN_ID is set" >&2
        exit 1
    fi
    ETH_CHAIN_ID_LINE="eth_chain_id: ${ETH_CHAIN_ID}"
    ETH_OBSERVER_CONFIG="eth_header_observer:
  enabled: true
  rpc_url: \"${ETH_RPC_URL}\"
  block_tag: \"${ETH_BLOCK_TAG:-finalized}\""
fi

WORKSPACE="${WORKSPACE:-/workspace}"
cd "$WORKSPACE"

debug "Generating genesis for $NUM_VALIDATORS validators (chain_id=$CHAIN_ID, base_ip=$BASE_IP)"

echo "=== Generating genesis for $NUM_VALIDATORS validators (chain_id=$CHAIN_ID) ==="

# Parse base IP to get the base octets and starting number
IFS='.' read -r ip1 ip2 ip3 ip4 <<< "$BASE_IP"

# Generate usernames array for layout
USERNAMES=""
for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    if [ -n "$USERNAMES" ]; then
        USERNAMES="${USERNAMES}, \"validator-${i}\""
    else
        USERNAMES="\"validator-${i}\""
    fi
done

# Step 1: Generate root account keys (for test-only faucet)
echo "Step 1/7: Generating root account keys..."
mkdir -p "root-account"
/usr/local/bin/aptos genesis generate-keys --output-dir "root-account" --assume-yes > "$APTOS_OUTPUT"

# Extract root public key for layout.yaml
ROOT_PUBLIC_KEY=$(grep "account_public_key:" root-account/public-keys.yaml | awk '{print $2}' | tr -d '"')
debug "Root account public key: $ROOT_PUBLIC_KEY"

# Step 2: Generate validator keys
echo "Step 2/7: Generating validator keys..."
for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    username="validator-${i}"
    mkdir -p "$username"
    /usr/local/bin/aptos genesis generate-keys --output-dir "$username" --assume-yes > "$APTOS_OUTPUT"
done

# Step 3: Create layout.yaml with generated root key
echo "Step 3/7: Creating layout.yaml..."
# SCOUT(#225) EDIT: add a `timelock_registrar:` line.
# SCOUT(#231) EDIT: add the epoch boundary values next to `epoch_duration_secs: 7200`.
cat > layout.yaml <<EOF
---
root_key: "${ROOT_PUBLIC_KEY}"
users: [${USERNAMES}]
chain_id: ${CHAIN_ID}
allow_new_validators: false
epoch_duration_secs: 7200
is_test: true
min_stake: 100000000000000
min_voting_threshold: 100000000000000
max_stake: 100000000000000000
recurring_lockup_duration_secs: 86400
required_proposer_stake: 100000000000000
rewards_apy_percentage: 10
voting_duration_secs: 43200
voting_power_increase_limit: 20
timelock_dkg_enabled: ${TIMELOCK_DKG_ENABLED}
${TIMELOCK_HASH_LINE}
${ETH_CHAIN_ID_LINE}
EOF

# Step 4: Set validator configurations
echo "Step 4/7: Setting validator configurations..."
mkdir -p genesis-repo

for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    username="validator-${i}"
    validator_ip="${ip1}.${ip2}.${ip3}.$((ip4 + i))"

    debug "Setting configuration for $username at IP $validator_ip"

    mkdir -p "genesis-repo/${username}"

    /usr/local/bin/aptos genesis set-validator-configuration \
        --username "$username" \
        --owner-public-identity-file "${username}/public-keys.yaml" \
        --validator-host "${validator_ip}:6180" \
        --full-node-host "${validator_ip}:6182" \
        --stake-amount "$STAKE_AMOUNT" \
        --commission-percentage 0 \
        --local-repository-dir genesis-repo > "$APTOS_OUTPUT"

    debug "Configured $username with validator-host=${validator_ip}:6180"
done

# Step 5: Copy layout to genesis repo and find framework
echo "Step 5/7: Setting up genesis repository..."
cp layout.yaml genesis-repo/

# Find and copy framework.mrb
# Priority: /framework.mrb (mounted by Docker) > standard paths > git repo search
FRAMEWORK_PATHS=(
    "/framework.mrb"
    "/aptos-framework/move/head.mrb"
    "/opt/aptos/framework/head.mrb"
    "/usr/local/share/aptos/framework/head.mrb"
)

FRAMEWORK_FOUND=false
for path in "${FRAMEWORK_PATHS[@]}"; do
    if [ -f "$path" ]; then
        debug "Found framework at: $path"
        cp "$path" genesis-repo/framework.mrb
        FRAMEWORK_FOUND=true
        break
    fi
done

# Fallback: search repository for any .mrb file
if [ "$FRAMEWORK_FOUND" = false ]; then
    debug "Searching repository for .mrb files"
    REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
    if [ -n "$REPO_ROOT" ]; then
        FOUND=$(find "$REPO_ROOT" -type f -name "*.mrb" | head -n 1)
        if [ -n "$FOUND" ]; then
            debug "Found fallback framework at: $FOUND"
            cp "$FOUND" genesis-repo/framework.mrb
            FRAMEWORK_FOUND=true
        fi
    fi
fi

if [ "$FRAMEWORK_FOUND" = false ]; then
    echo "ERROR: Could not find framework.mrb in any known location"
    exit 1
fi

# Step 6: Setup git structure (required by aptos genesis)
echo "Step 6/7: Finalizing genesis repository..."
/usr/local/bin/aptos genesis setup-git \
    --layout-file layout.yaml \
    --local-repository-dir genesis-repo > "$APTOS_OUTPUT"

# Step 7: Generate genesis blob and waypoint
echo "Step 7/7: Generating genesis.blob and waypoint..."
mkdir -p output
/usr/local/bin/aptos genesis generate-genesis \
    --local-repository-dir genesis-repo \
    --output-dir output > "$APTOS_OUTPUT"

# Save root account keys for TypeScript SDK faucet
cp root-account/private-keys.yaml output/root-account-private-keys.yaml

# Create node configs for each validator
# These are placed in each validator's directory and will be copied to /opt/aptos/var/etc/node-config.yaml
for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    username="validator-${i}"

    debug "Creating node config for $username in ${username}/node-config.yaml"

    cat > "${username}/node-config.yaml" << EOF
base:
  role: "validator"
  data_dir: "/opt/aptos/var/data"
  waypoint:
    from_file: "/opt/aptos/var/genesis/waypoint.txt"

execution:
  genesis_file_location: "/opt/aptos/var/genesis/genesis.blob"
  blockstm_v2_enabled: true

consensus:
  # Built into the validator image from atomica/timelock-params (see atomica/docker/Dockerfile).
  timelock_public_parameters_blob_path: "/opt/aptos/etc/timelock-public-parameters.bcs"
  safety_rules:
    service:
      type: "local"
    backend:
      type: "on_disk_storage"
      path: "/opt/aptos/var/data/safety-rules.bin"
    initial_safety_rules_config:
      from_file:
        identity_blob_path: "/opt/aptos/var/identity/validator-identity.yaml"
        waypoint:
          from_file: "/opt/aptos/var/genesis/waypoint.txt"

validator_network:
  discovery_method: "onchain"
  listen_address: "/ip4/0.0.0.0/tcp/6180"
  mutual_authentication: true
  identity:
    type: "from_file"
    path: "/opt/aptos/var/identity/validator-identity.yaml"

full_node_networks: []

api:
  enabled: true
  address: "0.0.0.0:8080"

storage:
  rocksdb_configs:
    enable_storage_sharding: false
  hot_state_config:
    max_items_per_shard: 250000
    refresh_interval_versions: 100000
    delete_on_restart: true
    compute_root_hash: true

${ETH_OBSERVER_CONFIG}
EOF

    debug "Created config for $username"
done

echo "=== Genesis generation complete! ==="
echo "Waypoint:"
cat output/waypoint.txt

#!/bin/bash
# Generate Aptos genesis on HOST (not in Docker)
# This script uses the host's aptos CLI to generate genesis artifacts
set -e

# Enable debug mode if ATOMICA_DEBUG_TESTNET is set
if [ -n "$ATOMICA_DEBUG_TESTNET" ]; then
    set -x
fi

NUM_VALIDATORS="${1:-4}"
CHAIN_ID="${2:-4}"
BASE_IP="${3:-172.19.0.10}"
STAKE_AMOUNT="100000000000000"  # 1M APT in octas

WORKSPACE="${WORKSPACE:-./genesis-workspace}"

# Clean and create workspace
rm -rf "$WORKSPACE"
mkdir -p "$WORKSPACE"
cd "$WORKSPACE"

echo "=== Generating genesis for $NUM_VALIDATORS validators (chain_id=$CHAIN_ID) ==="

# Parse base IP
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

# Step 1: Generate root account keys
echo "Step 1/7: Generating root account keys..."
mkdir -p "root-account"
aptos genesis generate-keys --output-dir "root-account" --assume-yes

# Extract root public key
ROOT_PUBLIC_KEY=$(grep "account_public_key:" root-account/public-keys.yaml | awk '{print $2}' | tr -d '"')
echo "  Root public key: $ROOT_PUBLIC_KEY"

# Step 2: Generate validator keys
echo "Step 2/7: Generating validator keys..."
for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    username="validator-${i}"
    mkdir -p "$username"
    aptos genesis generate-keys --output-dir "$username" --assume-yes
done

# Step 3: Create layout.yaml
echo "Step 3/7: Creating layout.yaml..."
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
EOF

# Step 4: Set validator configurations
echo "Step 4/7: Setting validator configurations..."
mkdir -p genesis-repo

for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    username="validator-${i}"
    validator_ip="${ip1}.${ip2}.${ip3}.$((ip4 + i))"

    echo "  Configuring $username at $validator_ip"

    mkdir -p "genesis-repo/${username}"

    aptos genesis set-validator-configuration \
        --username "$username" \
        --owner-public-identity-file "${username}/public-keys.yaml" \
        --validator-host "${validator_ip}:6180" \
        --full-node-host "${validator_ip}:6182" \
        --stake-amount "$STAKE_AMOUNT" \
        --commission-percentage 0 \
        --local-repository-dir genesis-repo
done

# Step 5: Copy layout and find framework
echo "Step 5/7: Setting up genesis repository..."
cp layout.yaml genesis-repo/

# Find framework.mrb - check multiple locations
# Use freshly built framework with IBE support
FRAMEWORK_PATHS=(
    "$(pwd)/framework-latest.mrb"
    "../../aptos-move/framework/releases/head.mrb"
    "$HOME/atomica/source/move-framework-fixtures/head.mrb"
    "/opt/aptos/framework/head.mrb"
    "$HOME/.move/head.mrb"
    "$(dirname $(which aptos))/../share/aptos/framework/head.mrb"
)

FRAMEWORK_FOUND=false
for path in "${FRAMEWORK_PATHS[@]}"; do
    if [ -f "$path" ]; then
        echo "  Found framework at: $path"
        cp "$path" genesis-repo/framework.mrb
        FRAMEWORK_FOUND=true
        break
    fi
done

if [ "$FRAMEWORK_FOUND" = false ]; then
    # Try to extract from docker image
    echo "  Extracting framework from docker image..."
    if docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest cat /opt/aptos/framework/head.mrb > genesis-repo/framework.mrb 2>/dev/null; then
        FRAMEWORK_FOUND=true
        echo "  Extracted framework from docker image"
    fi
fi

if [ "$FRAMEWORK_FOUND" = false ]; then
    echo "ERROR: Could not find framework.mrb"
    exit 1
fi

# Step 6: Setup git structure
echo "Step 6/7: Finalizing genesis repository..."
aptos genesis setup-git \
    --layout-file layout.yaml \
    --local-repository-dir genesis-repo

# Step 7: Generate genesis blob and waypoint
echo "Step 7/7: Generating genesis.blob and waypoint..."
mkdir -p output
aptos genesis generate-genesis \
    --local-repository-dir genesis-repo \
    --output-dir output

# Save root account keys
cp root-account/private-keys.yaml output/root-account-private-keys.yaml

# Create node configs for each validator
for i in $(seq 0 $((NUM_VALIDATORS - 1))); do
    username="validator-${i}"

    cat > "${username}/node-config.yaml" << EOF
base:
  role: "validator"
  data_dir: "/opt/aptos/var/data"
  waypoint:
    from_file: "/opt/aptos/var/genesis/waypoint.txt"

execution:
  genesis_file_location: "/opt/aptos/var/genesis/genesis.blob"

consensus:
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
EOF
done

echo "=== Genesis generation complete! ==="
echo "Waypoint:"
cat output/waypoint.txt

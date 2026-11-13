#!/usr/bin/env bash
# Timelock smoke test against a RUNNING network whose epoch already has a timelock key.
#
# What it does, over the REST API and the `aptos` CLI only:
#   1. waits for ibe_config::is_ready,
#   2. registers a timelock with a deadline a minute ahead,
#   3. encrypts three bids (V2 wrapper) to the timelock's master public key with the client tool,
#   4. checks that nothing is revealed before the deadline,
#   5. waits for the reveal, checks the key with e(dk, g2) = e(H1(id), mpk), decrypts every bid,
#   6. checks that a bid copied under another bidder's associated data fails and that the others still open,
#   7. (FORCE_EPOCH_CHANGE=1) registers a second timelock, forces an epoch change with the root profile
#      before its deadline and checks the status is expired with reason epoch-ended (1).
#
# The network must have the timelock DKG enabled (`timelock_dkg::enable`, then one epoch change). The Rust
# smoke test `testsuite/smoke-test/src/timelock_e2e.rs` shows the whole flow on a fresh swarm and has a
# companion test (`timelock_testnet_for_script`) that keeps such a swarm alive and prints the variables below.
#
# Variables:
#   REST_URL         node REST endpoint, for example http://127.0.0.1:8080/v1
#   REGISTRAR_KEY    hex private key of the on-chain timelock registrar (`ibe_config::timelock_registrar`).
#                    Only that address may register. Test networks set the root account 0xa550c18.
#   REGISTRAR_ADDRESS  address of the registrar (default 0xa550c18, the root account, whose address
#                    does not derive from its key)
#   BIDDER_KEYS      three comma separated hex private keys of funded accounts (bidder identities)
#   ROOT_KEY         hex private key of the root (core resources) account, only for FORCE_EPOCH_CHANGE=1
#   APTOS            path of the `aptos` CLI (default: aptos)
#   BID_TOOL         path of the built `timelock_bid` example (default: cargo run of crates/aptos-dkg)
#   DEADLINE_AHEAD_SECS  seconds between registration and deadline (default 60)
#   AUCTION_ID       auction id bound into the associated data (default 5)
#
# Exit status 0 only if every assertion passed. Each assertion prints one `ASSERT <name>: PASS|FAIL` line.
set -uo pipefail

REST_URL="${REST_URL:?set REST_URL}"
REGISTRAR_KEY="${REGISTRAR_KEY:?set REGISTRAR_KEY}"
REGISTRAR_ADDRESS="${REGISTRAR_ADDRESS:-0xa550c18}"
BIDDER_KEYS="${BIDDER_KEYS:?set BIDDER_KEYS (three keys, comma separated)}"
APTOS="${APTOS:-aptos}"
DEADLINE_AHEAD_SECS="${DEADLINE_AHEAD_SECS:-60}"
AUCTION_ID="${AUCTION_ID:-5}"
FORCE_EPOCH_CHANGE="${FORCE_EPOCH_CHANGE:-0}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REST_URL="${REST_URL%/}"
case "$REST_URL" in */v1) ;; *) REST_URL="$REST_URL/v1" ;; esac

if [ -n "${BID_TOOL:-}" ]; then
  bid_tool() { "$BID_TOOL" "$@"; }
else
  bid_tool() {
    GIT_LFS_SKIP_SMUDGE=1 cargo run --quiet --release --manifest-path "$ROOT/Cargo.toml" \
      -p aptos-dkg --example timelock_bid -- "$@"
  }
fi

FAILED=0
assert() { # name, status(0 = pass), detail
  if [ "$2" -eq 0 ]; then echo "ASSERT $1: PASS ($3)"; else echo "ASSERT $1: FAIL ($3)"; FAILED=1; fi
}

view() { # function, json array of args
  curl -sf -X POST "$REST_URL/view" -H 'content-type: application/json' \
    -d "{\"function\":\"$1\",\"type_arguments\":[],\"arguments\":${2:-[]}}"
}
chain_now_us() { view 0x1::timestamp::now_microseconds | jq -r '.[0]'; }

run_as() { # key, function, args...
  local key="$1" fn="$2"; shift 2
  "$APTOS" move run --private-key "$key" --url "$REST_URL" --assume-yes \
    --function-id "$fn" "$@" >/dev/null
}

echo "waiting for ibe_config::is_ready"
for _ in $(seq 1 120); do
  [ "$(view 0x1::ibe_config::is_ready | jq -r '.[0]' 2>/dev/null)" = "true" ] && break
  sleep 2
done
READY="$(view 0x1::ibe_config::is_ready | jq -r '.[0]')"
assert timelock-key-ready "$([ "$READY" = true ]; echo $?)" "is_ready=$READY"
[ "$READY" = true ] || exit 1

CHAIN_ID="$(curl -sf "$REST_URL" | jq -r '.chain_id')"
EPOCH="$(curl -sf "$REST_URL" | jq -r '.epoch')"
MPK="$(view 0x1::ibe_config::get_mpk | jq -r '.[0]')"

NOW="$(chain_now_us)"
DEADLINE=$((NOW + DEADLINE_AHEAD_SECS * 1000000))
TL="$(view 0x1::ibe_config::get_next_timelock_id | jq -r '.[0]')"
run_as "$REGISTRAR_KEY" 0x1::ibe_config::register_timelock --args "u64:$DEADLINE" \
  --sender-account "$REGISTRAR_ADDRESS"
TL_EPOCH="$(view 0x1::ibe_config::get_timelock_epoch "[\"$TL\"]" | jq -r '.[0]')"
ID="$(view 0x1::ibe_config::get_identity "[\"$TL\"]" | jq -r '.[0]')"
EXPECT_ID="0x$(bid_tool identity "$CHAIN_ID" "$TL_EPOCH" "$TL" "$DEADLINE")"
assert timelock-identity-epoch-bound "$([ "$ID" = "$EXPECT_ID" ]; echo $?)" "timelock $TL epoch $TL_EPOCH"

IFS=',' read -r K0 K1 K2 <<<"$BIDDER_KEYS"
bidder_addr() { # key -> 0x address (32 bytes hex)
  "$APTOS" init --profile "tmp-$2" --private-key "$1" --network custom --rest-url "$REST_URL" \
    --skip-faucet --assume-yes >/dev/null 2>&1
  "$APTOS" config show-profiles --profile "tmp-$2" 2>/dev/null | jq -r '.Result["tmp-'"$2"'"].account'
}
pad32() { local h="${1#0x}"; printf '%064s' "$h" | tr ' ' 0; }
A0="$(pad32 "$(bidder_addr "$K0" 0)")"
A1="$(pad32 "$(bidder_addr "$K1" 1)")"
A2="$(pad32 "$(bidder_addr "$K2" 2)")"

AD0="$(bid_tool ad "$CHAIN_ID" "$TL_EPOCH" "$TL" "$AUCTION_ID" "$A0" "$DEADLINE")"
AD1="$(bid_tool ad "$CHAIN_ID" "$TL_EPOCH" "$TL" "$AUCTION_ID" "$A1" "$DEADLINE")"
AD2="$(bid_tool ad "$CHAIN_ID" "$TL_EPOCH" "$TL" "$AUCTION_ID" "$A2" "$DEADLINE")"
plaintext() { python3 -c 'import struct,sys; print((struct.pack("<Q", int(sys.argv[1])) + bytes([int(sys.argv[2])]) * 32).hex())' "$1" "$2"; }
PT0="$(plaintext 1500 1)"; PT1="$(plaintext 2750 2)"; PT2="$(plaintext 900 3)"
CT0="$(bid_tool encrypt "$MPK" "$ID" "$AD0" "$PT0")"
CT1="$(bid_tool encrypt "$MPK" "$ID" "$AD1" "$PT1")"
CT2="$(bid_tool encrypt "$MPK" "$ID" "$AD2" "$PT2")"
bid_tool verify "$CT0" "$AD0" >/dev/null; assert bids-verify-at-submission $? "three V2 bids"

SEALED=0
if [ "$(chain_now_us)" -lt "$DEADLINE" ]; then
  [ "$(view 0x1::ibe_config::is_revealed "[\"$TL\"]" | jq -r '.[0]')" = false ] || SEALED=1
  view 0x1::ibe_config::get_decryption_key "[\"$TL\"]" >/dev/null 2>&1 && SEALED=1
fi
assert sealed-until-deadline "$SEALED" "not revealed and no key before the committed deadline"

echo "waiting for the reveal"
REV=false
for _ in $(seq 1 240); do
  REV="$(view 0x1::ibe_config::is_revealed "[\"$TL\"]" | jq -r '.[0]')"
  [ "$REV" = true ] && break
  sleep 2
done
assert reveal-after-threshold-shares "$([ "$REV" = true ]; echo $?)" "revealed=$REV"
if [ "$REV" = true ]; then
  DK="$(view 0x1::ibe_config::get_decryption_key "[\"$TL\"]" | jq -r '.[0]')"
  bid_tool check-key "$DK" "$ID" "$MPK" >/dev/null; assert revealed-key-verifies $? "e(dk,g2)=e(H1(id),mpk)"
  OK=0
  [ "$(bid_tool decrypt "$DK" "$ID" "$AD0" "$CT0")" = "$PT0" ] || OK=1
  [ "$(bid_tool decrypt "$DK" "$ID" "$AD1" "$CT1")" = "$PT1" ] || OK=1
  [ "$(bid_tool decrypt "$DK" "$ID" "$AD2" "$CT2")" = "$PT2" ] || OK=1
  assert bids-decrypt-with-revealed-key "$OK" "three bids"
  COPY=0
  bid_tool verify "$CT0" "$AD1" >/dev/null 2>&1 && COPY=1
  bid_tool decrypt "$DK" "$ID" "$AD1" "$CT0" >/dev/null 2>&1 && COPY=1
  [ "$(bid_tool decrypt "$DK" "$ID" "$AD2" "$CT2")" = "$PT2" ] || COPY=1
  assert copied-bid-fails-others-decrypt "$COPY" "bidder 1 replays the ciphertext of bidder 0"
fi

if [ "$FORCE_EPOCH_CHANGE" = 1 ]; then
  ROOT_KEY="${ROOT_KEY:?set ROOT_KEY for FORCE_EPOCH_CHANGE=1}"
  NOW="$(chain_now_us)"
  TL2="$(view 0x1::ibe_config::get_next_timelock_id | jq -r '.[0]')"
  run_as "$REGISTRAR_KEY" 0x1::ibe_config::register_timelock --args "u64:$((NOW + 600 * 1000000))" \
    --sender-account "$REGISTRAR_ADDRESS"
  EPOCH0="$(curl -sf "$REST_URL" | jq -r '.epoch')"
  # The root account is the core resources account 0xa550c18. Its balance is large and gas price 1 is accepted.
  run_as "$ROOT_KEY" 0x1::aptos_governance::force_end_epoch_test_only \
    --sender-account 0xa550c18 --max-gas 2000000 --gas-unit-price 1
  for _ in $(seq 1 300); do
    [ "$(curl -sf "$REST_URL" | jq -r '.epoch')" -gt "$EPOCH0" ] && break
    sleep 2
  done
  ST="$(view 0x1::ibe_config::get_status "[\"$TL2\"]" | jq -r '.[0]')"
  RS="$(view 0x1::ibe_config::get_expiry_reason "[\"$TL2\"]" | jq -r '.[0]')"
  assert epoch-change-expires-timelock "$([ "$ST" = 2 ] && [ "$RS" = 1 ]; echo $?)" "status=$ST reason=$RS (2,1 = expired, epoch-ended)"
fi

[ "$FAILED" -eq 0 ] && echo "TIMELOCK SMOKE TEST: PASS" || echo "TIMELOCK SMOKE TEST: FAIL"
exit "$FAILED"

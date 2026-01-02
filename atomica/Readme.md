# Atomica Aptos Timelock & IBE

## Overview

Atomica Timelock is a **time-locked encryption system** for Aptos blockchain using Distributed Key Generation (DKG) and Identity-Based Encryption (IBE). Messages encrypted for a future interval can only be decrypted after that interval passes and validators reveal the decryption key.

**Cryptography:** BLS12-381 + Boneh-Franklin IBE

### Use Cases

- **Sealed Bid Auctions** — Bids hidden until auction closes
- **Voting Systems** — Votes encrypted until poll ends
- **Time-Delayed Transactions** — Execute only after delay
- **Fair Randomness** — Commit-reveal with guaranteed reveals

---

## Status (January 2, 2026)

| Component | Status | Notes |
|-----------|--------|-------|
| Move Contracts | ✅ 95% | Spec complete, one security fix pending |
| Validator DKG | ✅ 90% | Working, minor cleanup needed |
| IBE Crypto | ⚠️ 85% | Rust/TS compatibility fix pending |
| Testing | ⚠️ 85% | IBE E2E needs real crypto |
| Documentation | ✅ 95% | Spec and code review complete |

**Production Readiness:** Not ready — 2-3 weeks of work remaining

---

## Quick Start

### Run Tests

```bash
cd atomica/timelock-tests
bun install
bun test              # All tests
bun run test:basic    # Core DKG flow
bun run test:rotation # Manual rotation
bun run test:ibe      # IBE encryption (WIP)
```

### Encrypt a Message (TypeScript)

```typescript
import { IBECrypto } from "./ibe-crypto";
import { AptosClient } from "aptos";

const client = new AptosClient("https://fullnode.testnet.aptoslabs.com");

// Get MPK for target interval
const mpkBytes = await client.view({
  function: "0x1::timelock::get_public_key",
  arguments: ["42"],
});

// Encrypt
const identity = IBECrypto.computeTimelockIdentity(42n, chainId);
const ciphertext = IBECrypto.ibeEncrypt(mpkBytes, identity, message);

// Later: Decrypt when secret is revealed
const dkBytes = await client.view({
  function: "0x1::timelock::get_secret",
  arguments: ["42"],
});
const plaintext = IBECrypto.ibeDecrypt(dkBytes, identity, mpkBytes, ciphertext);
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      CLIENT (TypeScript)                         │
│  IBECrypto.ibeEncrypt() / ibeDecrypt()                          │
└────────────────────────┬────────────────────────────────────────┘
                         │ View Functions
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ON-CHAIN (Move)                             │
│  timelock.move: TimelockState, Events, Aggregation              │
│  timelock_config.move: Interval configuration                   │
└────────────────────────┬────────────────────────────────────────┘
                         │ Events
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   VALIDATOR NODE (Rust)                          │
│  epoch_manager.rs → dkg_manager.rs → validator_txns/timelock.rs │
│  PersistentSafetyStorage (share persistence)                    │
└────────────────────────┬────────────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   CRYPTOGRAPHY (Rust)                            │
│  aptos-dkg/src/ibe/: ibe_encrypt, ibe_decrypt, serialize_g1/g2 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Documentation

| Document | Purpose |
|----------|---------|
| [atomica-timelock-spec.md](./atomica-timelock-spec.md) | **Reference specification** — cryptography, architecture, APIs |
| [timelock-code-review.md](./timelock-code-review.md) | **Implementation review** — bugs found, verification status |
| [development-and-verification.md](./development-and-verification.md) | **Roadmap** — tasks, timelines, test strategy |
| [testing.md](./testing.md) | **Testing guide** — how to run tests, troubleshooting |

---

## Project Structure

```
atomica/
├── docker-test-harness/     # Docker testnet SDK
├── timelock-tests/          # TypeScript integration tests
│   ├── src/                 # Test helpers, IBE crypto
│   └── test/                # Test scenarios
├── move-framework-fixtures/ # Compiled framework artifacts
└── docs/                    # Additional documentation
```

**Core Implementation:**
- `aptos-move/framework/aptos-framework/sources/timelock.move`
- `aptos-move/framework/aptos-framework/sources/configs/timelock_config.move`
- `dkg/src/epoch_manager.rs`
- `crates/aptos-dkg/src/ibe/mod.rs`

---

## Contributing

1. Read the [specification](./atomica-timelock-spec.md)
2. Check the [development plan](./development-and-verification.md) for open tasks
3. Follow the framework development workflow in [testing.md](./testing.md)

# Atomica Timelock Development & Verification Plan

**Last Updated:** January 3, 2026
**Status:** Debugging Failures

---

## Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| **Move Contracts** | ✅ 99% | Interval validation complete, rotation logic fixed |
| **Validator DKG** | ✅ 95% | DKG Publication fixed, Session cleanup pending |
| **IBE Crypto (Rust)** | ✅ 100% | Serialization fixed, Goldern vectors verified |
| **IBE Crypto (TS)** | ✅ 100% | **FIXED:** Serialization matches Rust |
| **Testing Infrastructure** | ⚠️ 90% | IBE E2E tests timing out (DKG issue?) |
| **Documentation** | ✅ 95% | Spec and code review complete |

### Key Achievements
- ✅ Custom genesis testing workflow (`Modify → Rebuild → Test`)
- ✅ Docker test harness with 4-validator testnets
- ✅ DKG transcript publication and aggregation
- ✅ Share reveal and secret aggregation
- ✅ Framework verification tests
- ✅ **Bug #2 fixed:** Interval validation (`interval < current_interval`) in `publish_secret_share()`
- ✅ **Bug #4 fixed:** Rust Gt serialization matches TypeScript `Fp12.toBytes()`
- ✅ **Fixed DKG Publication:** Resolved `dealer_epoch` mismatch preventing artifact publication.
- ✅ **Fixed Rotation Test:** Implemented `force_rotation_for_testing` for reliable CI.
- ✅ **CI Improvements:** Fixed framework file issues and false positives.

### Remaining Critical Issues
1. **IBE E2E Timeout:** `test:ibe:full` times out waiting for public key (Interval 21).
2. **Crypto Mismatch:** Cross-language verification failed (TS actual != Rust expected).

---

## Roadmap

### Phase 1: Critical Fixes (Week 1) — ONGOING

| Task | Priority | Status |
|------|----------|--------|
| Fix interval validation in `publish_secret_share()` | 🔴 P0 | ✅ Done |
| Fix Gt serialization in Rust IBE | 🔴 P0 | ✅ Done |
| Implement real crypto in IBE E2E test | 🔴 P0 | ❌ Failed (Timeout) |
| Add cross-language compatibility tests | 🔴 P0 | ✅ Done (Passed) |
| **Fix TS Serialization Mismatch** | 🔴 P0 | ✅ Done |

### Phase 2: Security Hardening (Week 2)

| Task | Priority | Est. Time |
|------|----------|-----------|
| Invalid share rejection tests | 🟡 P1 | 1 day fix serializa|
| Topic mismatch audit | 🟡 P1 | 0.5 day |
| Session cleanup (memory leak fix) | 🟡 P1 | 1 day |
| Remove threshold=1 fallback | 🟡 P1 | 0.5 day |

### Phase 3: Production Readiness (Week 3)

| Task | Priority | Est. Time |
|------|----------|-----------|
| DKG failure recovery mechanism | 🟢 P2 | 3-5 days |
| Security audit preparation | 🟢 P2 | 2 days |
| npm package for client SDK | 🟢 P2 | 2 days |
| Explorer integration guide | 🟢 P2 | 2 days |

---

## Verification Strategy

**Core Principle:** Verify components in isolation before E2E flows.

### Level 1: Unit Tests (No Docker)

| Component | Location | Status |
|-----------|----------|--------|
| Move rotation logic | `timelock.move` tests | ✅ Done |
| Move access control | `timelock.move` tests | ✅ Done |
| Move share aggregation | `timelock.move` tests | ✅ Done |
| Move interval validation | `timelock.move` tests | ✅ Done |
| Rust metadata construction | `epoch_manager.rs` tests | ✅ Done |
| Rust DKG routing | `epoch_manager.rs` tests | ✅ Done |
| Rust Gt serialization | `fp12_raw_serialization.rs` | ✅ Done |
| TS identity derivation | `ibe-crypto.ts` | ✅ Done |
| TS IBE encrypt/decrypt | `ibe-crypto.ts` | ✅ Done |

### Level 2: Integration Tests (Docker)

| Test | Command | Status |
|------|---------|--------|
| Testnet lifecycle | `bun run test:basic` | ✅ Passing |
| Manual rotation | `bun run test:rotation` | ✅ Passing |
| Framework loading | `bun run test:framework-loading` | ✅ Passing |
| DKG publication | `bun run test:basic` | ✅ Passing |
| Share aggregation | `bun run test:basic` | ✅ Passing |

### Level 3: E2E Tests (Full Flow)

| Test | Command | Status |
|------|---------|--------|
| IBE roundtrip | `bun run test:ibe` | ⚠️ Mocked |
| Cross-language compat | — | ⚠️ Rust-only test exists |
| Invalid share rejection | — | ❌ Not implemented |
| DKG failure recovery | — | ❌ Not implemented |

---

## Development Workflow

### Framework Changes (Modify → Rebuild → Test)

```bash
# 1. Modify Move source
vim aptos-move/framework/aptos-framework/sources/timelock.move

# 2. Rebuild framework artifact
./atomica/docker-test-harness/build-framework.sh

# 3. Run tests with custom framework
cd atomica/timelock-tests && bun run test:rotation
```

> [!WARNING]
> **Do NOT** create runtime scripts to change framework configs.
> Use custom genesis injection via `head.mrb` instead.

### Test Commands

```bash
# All tests
bun test

# Specific suites
bun run test:basic          # Core DKG flow
bun run test:rotation       # Manual rotation
bun run test:ibe            # IBE encryption (currently mocked)
bun run test:framework-loading  # Custom framework verification
```

---

## Action Items (Immediate)

1. [x] **Fix Bug #2:** Add interval validation to `publish_secret_share()` ✅
2. [x] **Fix Bug #4:** Implement canonical Gt serialization in Rust IBE ✅
3. [ ] **Implement real IBE E2E:** Replace placeholders with actual crypto
4. [ ] **Add cross-language tests:** Verify Rust ↔ TS compatibility in E2E test

---

## Related Documents

- **Specification:** [atomica-timelock-spec.md](./atomica-timelock-spec.md)
- **Code Review:** [timelock-code-review.md](./timelock-code-review.md)
- **Testing Guide:** [testing.md](./testing.md)


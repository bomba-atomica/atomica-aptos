# Atomica Timelock Development & Verification Plan

**Last Updated:** January 2, 2026
**Status:** Active Development

---

## Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| **Move Contracts** | ✅ 95% | Needs interval validation fix |
| **Validator DKG** | ✅ 90% | Session cleanup pending |
| **IBE Crypto (Rust)** | ⚠️ 80% | Gt serialization fix needed |
| **IBE Crypto (TS)** | ✅ 95% | Complete, tested locally |
| **Testing Infrastructure** | ✅ 85% | IBE E2E tests need real crypto |
| **Documentation** | ✅ 90% | Spec and code review complete |

### Key Achievements
- ✅ Custom genesis testing workflow (`Modify → Rebuild → Test`)
- ✅ Docker test harness with 4-validator testnets
- ✅ DKG transcript publication and aggregation
- ✅ Share reveal and secret aggregation
- ✅ Framework verification tests

### Known Critical Issues
1. **Bug #2:** Missing `interval < current_interval` validation (security)
2. **Bug #4:** Rust/TS Gt serialization mismatch (interop)
3. **IBE E2E tests use placeholders** (quality)

---

## Roadmap

### Phase 1: Critical Fixes (Week 1)

| Task | Priority | Est. Time | Owner |
|------|----------|-----------|-------|
| Fix interval validation in `publish_secret_share()` | 🔴 P0 | 0.5 day | — |
| Fix Gt serialization in Rust IBE | 🔴 P0 | 2-3 days | — |
| Implement real crypto in IBE E2E test | 🔴 P0 | 2 days | — |
| Add cross-language compatibility tests | 🔴 P0 | 1 day | — |

### Phase 2: Security Hardening (Week 2)

| Task | Priority | Est. Time |
|------|----------|-----------|
| Invalid share rejection tests | 🟡 P1 | 1 day |
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
| Move share aggregation | `timelock.move` tests | ⚠️ Partial |
| Rust metadata construction | `epoch_manager.rs` tests | ✅ Done |
| Rust DKG routing | `epoch_manager.rs` tests | ✅ Done |
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
| Cross-language compat | — | ❌ Not implemented |
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

1. [ ] **Fix Bug #2:** Add interval validation to `publish_secret_share()`
2. [ ] **Fix Bug #4:** Implement canonical Gt serialization in Rust IBE
3. [ ] **Implement real IBE E2E:** Replace placeholders with actual crypto
4. [ ] **Add cross-language tests:** Verify Rust ↔ TS compatibility

---

## Related Documents

- **Specification:** [atomica-timelock-spec.md](./atomica-timelock-spec.md)
- **Code Review:** [timelock-code-review.md](./timelock-code-review.md)
- **Testing Guide:** [testing.md](./testing.md)

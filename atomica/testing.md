# Atomica Timelock Testing Guide

## Prerequisites

- **Bun runtime** (for TypeScript tests)
- **Docker** (for testnet containers)

---

## Test Commands

```bash
cd atomica/timelock-tests

# Install dependencies
bun install

# Run all tests
bun test

# Specific suites
bun run test:basic              # Core DKG flow
bun run test:rotation           # Manual rotation trigger
bun run test:ibe                # IBE encryption (WIP)
bun run test:framework-loading  # Custom framework verification
```

---

## Framework Development Workflow

When modifying Move framework code, follow this loop:

```bash
# 1. Modify Move sources
vim aptos-move/framework/aptos-framework/sources/timelock.move

# 2. Rebuild framework artifact
./atomica/docker-test-harness/build-framework.sh

# 3. Run tests (uses custom head.mrb)
cd atomica/timelock-tests && bun run test:rotation
```

> [!WARNING]
> Do NOT create runtime scripts to change framework configs.
> Use custom genesis injection via `head.mrb` instead.

---

## Test Architecture

```
timelock-tests/
├── src/
│   ├── index.ts              # Exports
│   ├── queries.ts            # Blockchain queries
│   ├── transactions.ts       # Transaction builders
│   ├── waiters.ts            # Polling utilities
│   └── ibe-crypto.ts         # IBE cryptography
└── test/
    ├── basic-flow.test.ts    # Core DKG tests
    ├── ibe-e2e.test.ts       # IBE roundtrip
    └── manual-rotation.test.ts
```

**Key Components:**
- `docker-test-harness/` — Manages 4-validator Docker testnets
- `move-framework-fixtures/head.mrb` — Compiled framework for custom genesis

---

## Troubleshooting

### Test Timeout
Increase timeout in `bun.config.ts` or add `--timeout 300000` flag.

### Framework Not Loading
Check that `head.mrb` exists and Docker volumes are correctly mounted.

### Docker Instability
Restart Docker daemon: `docker restart`

---

## Test Coverage

| Test | Status |
|------|--------|
| ✅ DKG transcript publication | Passing |
| ✅ Share reveal and aggregation | Passing |
| ✅ Manual rotation trigger | Passing |
| ✅ Custom framework loading | Passing |
| ⚠️ IBE encrypt/decrypt | Mocked (needs real crypto) |
| ❌ Invalid share rejection | Not implemented |
| ❌ DKG failure recovery | Not implemented |

See [development-and-verification.md](./development-and-verification.md) for roadmap.

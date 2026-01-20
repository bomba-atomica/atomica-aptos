# Atomica Timelock Documentation Index

This directory contains documentation for the Atomica timelock encryption system.

## Quick Start

**New to the project?** Start here:

1. [timelock-master-plan.md](timelock-master-plan.md) - Consolidated overview
2. [definitions.md](definitions.md) - Core terminology

## Documentation Map

### Architecture & Planning

| Document                                                                         | Purpose                                   | Lines |
| -------------------------------------------------------------------------------- | ----------------------------------------- | ----- |
| [timelock-master-plan.md](timelock-master-plan.md)                               | **START HERE** - Consolidated master plan | 362   |
| [implementation-plan-unified-dkg-ibe.md](implementation-plan-unified-dkg-ibe.md) | Detailed phase-by-phase plan (v2.13)      | 474   |
| [timelock-ibe-elgamal-plan.md](timelock-ibe-elgamal-plan.md)                     | Legacy IBE + ElGamal plan                 | 328   |
| [adr-001-dual-output-dkg.md](adr-001-dual-output-dkg.md)                         | Architecture decision record              | 377   |

### Reference

| Document                                         | Purpose                                 | Lines |
| ------------------------------------------------ | --------------------------------------- | ----- |
| [definitions.md](definitions.md)                 | Core terminology and concepts           | 366   |
| [timelock-evaluation.md](timelock-evaluation.md) | Implementation status + risk assessment | 225   |

### Technical Deep Dives

| Document                                                                                         | Purpose                       |
| ------------------------------------------------------------------------------------------------ | ----------------------------- |
| [technical/chunked-elgamal-scalar-generation.md](technical/chunked-elgamal-scalar-generation.md) | Scalar generation details     |
| [technical/prior-art-aptos-apache.md](technical/prior-art-aptos-apache.md)                       | Prior art research            |
| [technical/prior-art-aptos-bibe-apache.md](technical/prior-art-aptos-bibe-apache.md)             | Prior art: Boneh-Franklin IBE |

### Developer Guides

| Document                                                                                             | Purpose                    |
| ---------------------------------------------------------------------------------------------------- | -------------------------- |
| [developer-guides/testing.md](developer-guides/testing.md)                                           | Testing strategy and tools |
| [developer-guides/smoke-test-debugging.md](developer-guides/smoke-test-debugging.md)                 | Debugging failing tests    |
| [developer-guides/docker-testnet.md](developer-guides/docker-testnet.md)                             | Local testnet setup        |
| [developer-guides/code-review-guide.md](developer-guides/code-review-guide.md)                       | Code review checklist      |
| [developer-guides/validator-event-subscription.md](developer-guides/validator-event-subscription.md) | Event handling             |

### Agent Prompts (Development Tasks)

| Document                                             | Purpose                              |
| ---------------------------------------------------- | ------------------------------------ |
| [agent-prompt-phase-1e.md](agent-prompt-phase-1e.md) | Phase 1E: Encrypt/decrypt smoke test |
| [agent-prompt-phase-3.md](agent-prompt-phase-3.md)   | Phase 3: Timelock registry           |
| [next-agent-prompt.md](next-agent-prompt.md)         | Next development tasks               |

### Product Specs

| Document                                                                       | Purpose                    |
| ------------------------------------------------------------------------------ | -------------------------- |
| [product-spec/atomica-timelock-spec.md](product-spec/atomica-timelock-spec.md) | Full product specification |
| [product-spec/dkg-overview.md](product-spec/dkg-overview.md)                   | DKG overview               |

## Architecture Overview

```
InputSecret (scalar a)
        │
        ├─────────────────────┬────────────────────────────┐
        │                     │                            │
        ▼                     ▼                            ▼
   ┌─────────┐      ┌─────────────────────┐      ┌─────────────────┐
   │DAS PVSS │      │Chunked Lifted       │      │   MPK = g2^a    │
   │  (G1)   │      │ElGamal PVSS (Scalar)│      │  (on-chain)     │
   └────┬────┘      └──────────┬──────────┘      └─────────────────┘
        │                      │
        ▼                      ▼
   WVUF/Randomness        IBE Decryption Key
```

## Current Status

**Last Updated:** January 20, 2026  
**Branch:** `timelock-vpss`  
**Primary Doc:** [timelock-master-plan.md](timelock-master-plan.md)

### Implementation Progress

| Component                      | Status       |
| ------------------------------ | ------------ |
| IBE Cryptography               | ✅ Complete  |
| Scalar ElGamal PVSS            | ✅ Complete  |
| DKG Protocol                   | ✅ Complete  |
| Validator Services             | ✅ Complete  |
| Move Contracts                 | ✅ Complete  |
| Tests                          | ✅ 30+ tests |
| **Phase 5.1 (Linear Pairing)** | 🔲 Pending   |

## Contributing

1. Update [timelock-master-plan.md](timelock-master-plan.md) for major changes
2. Update [definitions.md](definitions.md) for new terminology
3. Add technical deep dives to `technical/` folder
4. Add product specs to `product-spec/` folder

## Related Links

- [GitHub Repository](https://github.com/bomba-atomica/atomica-aptos)
- [Implementation Code](../crates/aptos-dkg/src/ibe/)
- [Tests](../testsuite/smoke-test/src/timelock/)

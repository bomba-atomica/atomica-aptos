### Memo: Encrypted Mempool / IBE Designs in Aptos

**Context.**
Aptos’ encrypted mempool (fptx) requires a mechanism by which clients can encrypt transactions such that contents are hidden prior to ordering, while validators can jointly decrypt only at the protocol-defined time. Two viable constructions exist: (1) a *minimal IBE-like scheme* built directly on existing validator BLS keys, and (2) a *DKG-based threshold IBE scheme* that introduces ephemeral collective keys. Both satisfy correctness; they differ in security envelope and operational complexity.

---

## 1. Minimal IBE-like construction (validator BLS only)

**Design.**
Each validator already has a long-term BLS keypair ((sk_i, pk_i)). Encryption targets a public identity
[
ID = H(\text{chain} ,|, \text{epoch} ,|, \text{round} ,|, \text{batch} ,|, R_e)
]
where (R_e) is epoch-specific consensus entropy. A client encrypts using pairings with the *aggregated validator public key*. After ordering, validators locally compute BLS signatures (\sigma_i = \text{Sign}(sk_i, ID)). Once a threshold of signatures is published, they are aggregated to produce a value that enables decryption.

There is no new secret key: the “decryption key” is an aggregated BLS signature on a time-scoped identity.

**Properties.**

* Anyone can encrypt without interaction.
* Decryption material does not exist until validators sign.
* Time binding is enforced via consensus entropy in `ID`.

**Pros.**

* **Minimal machinery**: no DKG, no new key lifecycle.
* **Low latency and complexity**: leverages existing BLS infra.
* **Easy integration** with current consensus and validator tooling.
* **Sufficient for basic encrypted mempool semantics**.

**Cons.**

* **Consensus keys = decryption authority**: increases blast radius.
* **Limited forward secrecy**: if ≥ t validators collude or are compromised later, they can decrypt all batches they previously signed.
* **No cryptographic erasure**: decryption capability is logically time-scoped but not cryptographically destroyed.
* Harder to extend cleanly to other threshold-crypto features without reuse risk.

---

## 2. DKG-based threshold IBE construction

**Design.**
At each epoch (or rotation interval), validators run a DKG/VPSS protocol to generate a *threshold BLS keypair* ((s, PK)), where:

* (s) is a group secret never materialized,
* each validator holds only a share (s_i),
* (PK = g^s) is public.

Clients encrypt against (PK) and an epoch-scoped identity. After ordering, validators produce partial decryptions using their shares; these are combined to enable decryption. At epoch end, shares are discarded.

**Properties.**

* Decryption authority is independent of validator identity keys.
* Secrets are epoch-scoped and ephemeral.

**Pros.**

* **Strong forward secrecy**: past epochs cannot be decrypted after share erasure.
* **Key separation**: compromise of consensus BLS keys does not imply decryption power.
* **Cleaner abstraction**: encrypted mempool is just one consumer of a general threshold-crypto service.
* **Extensible** to randomness beacons, sealed-bid auctions, private inputs, etc.

**Cons.**

* **Higher complexity**: DKG/VPSS protocols, share verification, churn handling.
* **Operational overhead**: extra rounds, state, and failure modes.
* **Latency risk** if DKG is not carefully amortized or pipelined.
* More code surface and audit burden.

---

## 3. Summary

Both constructions are correct and viable. The **minimal BLS-only approach** is attractive for simplicity and early deployment, and already achieves MEV-resistant encrypted mempools. The **DKG-based approach** is not strictly necessary for fptx correctness, but provides materially stronger forward-security and cleaner long-term architecture. Aptos’ apparent choice to invest in DKG/VPSS reflects a platform decision: building reusable threshold cryptography infrastructure rather than a single-purpose solution.

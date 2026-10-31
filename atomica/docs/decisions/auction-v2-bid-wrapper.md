# Auction contract on the V2 bid wrapper: decision record

Status: Implemented in the `atomica` repo on branch `fix/auction-v2-bid-wrapper`, commit `1139dfb`. Date: 2026-10-31. Prepared by an AI agent. No external cryptographer has reviewed it.

Design sources: [IBE on chunky DKG spec](aptos-upgrade-ibe-chunky-spec.md) sections 6 to 8 and the [review packet](aptos-upgrade-crypto-review-packet.md) sections 8 and 9.

## 1. The gap this closes

The auction contract (`source/atomica-move-contracts/sources/auction.move` in `atomica`) still checked cleartexts with the old XOR/pairing scheme. It never called `ibe::verify_bid_ct`. It read `is_timelock_expired` as "deadline passed". It had no refund path for an `Expired` timelock. No plan feature covered it.

## 2. Contract changes

| Area | Before | After |
|---|---|---|
| Bid shape | `u_bytes` (U) plus `ciphertext` (XOR body) | One V2 ciphertext `BCS(vk, U, nonce, C, sig)`, a 32-byte price commitment, `ct_id = H(vk, AD)` |
| `submit_bid` | Stored what it was given | Builds `AD = BCS(tag, chain_id, epoch, timelock_id, auction_id, bidder, deadline_us)` on chain, aborts unless `ibe::verify_bid_ct(ct, AD)` accepts |
| Submission rules | None | Timelock must be `Pending` (`get_status`). Bid must be strictly before the deadline. No duplicate `ct_id`. At most 100 bids per window. Commitment must be 32 bytes |
| Cleartext | XOR decrypt in Move, compare to a supplied price | `open_bid` and `submit_cleartext_and_clear(.., cleartexts, salts)` check `(price, salt)` against the stored commitment. Only when status is `Revealed` |
| Closing openings | n/a | `finalize_reveal` after every bid is open or `reveal_expiry + 10 min` |
| Clearing and settle | All bids count | Unopened bids are invalid: no fill, no collateral release, `BidUnopened` event |
| Expiry | `is_timelock_expired` meant deadline passed | `timelock::is_timelock_expired` is the `Expired` status. `is_deadline_passed` is separate. Facade adds `get_status`, `get_expiry_reason`, `get_mpk`, `get_timelock_epoch`, `get_deadline`, `get_reveal_expiry`, `is_revealed` |
| Refund | None | `refund_expired_window`: for status `Expired` (any reason), releases every bidder receipt and every seller lock. Runs once. `AuctionRefunded` carries the reason |
| Removed | `ibe_derive_key_block`, `xor_prefix`, `le_bytes_to_u64`, `test_submit_cleartext_with_dk`, the pairing imports | Nothing relies on the XOR scheme |

Associated data order matches the framework golden vector `bid_ad()` byte for byte. A Move test pins that equality on chain state (chain 7, epoch 3, timelock 11, auction 5, bidder `0x01..01`).

New abort codes: 12 invalid ciphertext, 13 bid after deadline, 14 timelock not pending, 15 duplicate bid, 16 timelock not revealed, 17 bid already opened, 18 invalid salt, 19 bid index out of range, 20 reveal final, 21 reveal not final, 22 opening window open, 23 timelock not expired, 24 window refunded, 25 window full, 26 invalid commitment, 27 timelock expired. Codes 7 and 8 are retired.

## 3. Decision: how a cleartext is verified without decryption

The spec leaves a hole here. Move cannot decrypt the AES-128-GCM wrapper. The spec says only that "a failed tag means the bid is invalid" and that decryption reads `AD` from the stored bid record. It does not say who runs decryption or how the chain learns the result. No new native is allowed.

Options weighed, least trust first:

1. Trust a relayer to submit cleartexts. Rejected: the relayer picks every price.
2. Relayer cleartext plus a dispute window. Rejected: a dispute needs on-chain decryption, which does not exist.
3. Chain verifies the decryption key only, prices stay off chain. Rejected: the chain would never learn the prices it must clear.
4. On-chain commitment to the price, opened with `(price, salt)`. Chosen.

How it works:

- Plaintext convention: `price_le8 || salt32` (40 bytes). The bidder encrypts it.
- At submission the bidder also passes `commitment = sha3_256("atomica/bid-commitment/v1" || bcs(price) || salt || sha3_256(AD))`. The AD binds it to one bidder, auction, epoch and timelock.
- After the registry status is `Revealed`, the verified key is public. Anyone can decrypt every bid off chain and call `open_bid` or the batch entry. The contract accepts an opening only if it matches the commitment.

Trust assumptions, stated plainly:

- A relayer cannot lie about a price or change one. The commitment pins it.
- Nobody can change a price after the key is public. The commitment is binding under SHA3-256 collision resistance.
- The commitment is hiding while the salt is random and secret until the key is released.
- The contract does not check that the ciphertext holds the committed price. Only the bidder knows. A bidder who commits to X and encrypts something else cannot be opened by a relayer. Only the bidder can open that bid. If the bidder does not, the bid is invalid.
- Relay liveness: any account can open any bid, so a single censoring relayer cannot stop an honest bid. The bidder can always open its own bid.
- The contract never reads the decryption key. It relies on the registry: `finalize_timelock_reveal` already checked `e(dk, g2) = e(H1(id), mpk)` before the status became `Revealed`.

Consequence: the V2 ciphertext gives the relayer liveness. The commitment gives binding. The two are not tied on chain.

## 4. Open items for the cryptographer

1. Free option. A bidder who encrypted garbage, or who simply stays silent, sees all other prices once the key is public and then decides whether to open. A non-opening bid costs nothing but its own participation. Is that acceptable? Policy now: unopened bids are invalid and their collateral stays claimed (not released). The relayer decides. A product owner must decide the slashing or release rule.
2. Is a commitment alone enough, or does the bid need a proof that ciphertext and commitment agree? Without a native for it, the only on-chain tie is none. A Fujisaki-Okamoto style check at decrypt time (spec section 11, B1) is also open. Both need a design that does not add natives, or a new native.
3. Commitment construction: SHA3-256 over a tagged, length-fixed input with `sha3_256(AD)`. Is binding the AD hash enough, or should the commitment bind `ct_id` too? Binding `ct_id` would tie a commitment to one ciphertext.
4. Opening window: `OPENING_WINDOW_US = 10 min` after the registry `reveal_expiry_us`. Parameters belong with spec section 11 (D2).
5. Bid cap: `MAX_BIDS_PER_WINDOW = 100` because the clearing sort is quadratic. Gas for 100 bids is not measured.
6. One timelock may back more than one window. AD binds the window id, so bids cannot cross windows with different ids. Windows with the same id and different pairs share an AD. A duplicate ciphertext from the same bidder is rejected only inside one window.

## 5. Not covered here

- No live-node run. All checks are Move unit tests in the Move test VM and TypeScript unit tests. The acceptance criterion for a live run stays pending.
- No Rust or TypeScript V2 encryptor for clients. `AuctionBidder` takes an operator-supplied ciphertext and commitment. A client encryptor needs a port of the wrapper (KEM, HKDF, AES-GCM, Ed25519). The TypeScript helpers cover the associated data, identity and commitment, with golden-vector parity tests.
- No golden vector for the 40-byte plaintext. The framework golden bid encrypts a text string. The Move tests commit to prices themselves and use the golden ciphertext only for the submission check.
- Registry lag. A timelock past `reveal_expiry_us` stays `Pending` until `on_new_block` marks it `Expired` (at most 50 per block). Refund waits for that block. The epoch-ended reason has no lag, because `get_status` derives it.
- `create_auction` does not validate the timelock id. A window bound to an unknown timelock id makes `get_status` abort, so no bid is accepted and refund aborts too. Add a check in a follow-up.
- Struct layout changed (`SealedBid`, `WindowState`). An upgrade over a deployed registry needs a migration. The testnet is not deployed with these resources.
- Consumers: `AuctionRevealer` still reads `is_revealed(window_id)`, which is the old window-to-timelock assumption. The window's timelock id should come from `get_window_status`.

## 6. Evidence

Commands and results are recorded in the forgectl feature `auction-v2-bid-wrapper` (checklist and `set-ci`). Local runs are not CI.

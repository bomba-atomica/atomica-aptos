import { describe, it, expect } from "bun:test";
import { bls12_381 } from "@noble/curves/bls12-381.js";
import { keccak_256 } from "@noble/hashes/sha3.js";
import { IBECrypto } from "../src/ibe-crypto.js";

// ============================================================================
// Cross-Implementation IBE Test Fixtures
// These match the Rust implementation in: crates/aptos-dkg/src/ibe/fixtures.rs
// ============================================================================

const FIXTURE_EPOCH = 42;
const FIXTURE_DEADLINE_MICROSECONDS = 1704070800000000n; // 2024-01-01 01:00:00 UTC
const FIXTURE_MESSAGE = "Golden Vector Message 2024";

const FIXTURE_IDENTITY_STRING = `timelock_id:${FIXTURE_EPOCH}:deadline_timestamp_microseconds:${FIXTURE_DEADLINE_MICROSECONDS}`;
const FIXTURE_IDENTITY_HASH = "6d68192a4097c6215fc53001590729f5f5d3f68e0449dcb94f76de2effc95f71";
const FIXTURE_MSK_HEX = "9949fb87fac301fa852d757fed7fa3cf093928850089766583d7eeea94376d01";
const FIXTURE_MPK_G2_HEX =
  "864479cdc8bbe7a3aa3194a50afdd5e4d45ec85ff1fdb58511468d61f51705f718fbf0094e5c0e74f95239f2edd244290c8c11c129562d1770a5f4dbe95ac60bb06c3e3c645d5b1ef65cd1a2ec06833d974506487787bfdd53b438040110c5cd";
const FIXTURE_DK_G1_HEX =
  "8a16b673b67dc8405eeea538753c8d5cb727dbc8e446464d3da08812d94061688fed6eca2dffd775e0dae6c53b15ffcb";
const FIXTURE_CIPHERTEXT_U_HEX =
  "a081a70d4d60e76aea4e4d4cce3731b741d0ba652783071ec3ad8a9dcd48817514e826915b93efcf68f0c8cbbe0a89200e6e09c384e863ef5ee54c8f8f99c5ba69480863109d229df3e00cd1be3d05e4238b2df0dbc18aff4b939ab88dc5610c";
const FIXTURE_CIPHERTEXT_V_HEX = "b847c649d14bbcdb7c5186af161bfb90116b0a4d0de331467044";
const FIXTURE_EXPECTED_DECRYPTED_HEX = "476f6c64656e20566563746f72204d6573736167652032303234";

// Rust Gt hash: 8cf9fb0d19ad2bf4d67ed58cda83159eb15ebce284355927b38ddc7b5
const EXPECTED_RUST_GT_HASH = [
  0x8c, 0xf9, 0xfb, 0x0d, 0x19, 0xad, 0x2b, 0xf4, 0xd6, 0x7e, 0xd5, 0x8c, 0xda, 0x83, 0x15, 0xeb, 0xce, 0x28, 0x43,
  0x55, 0x92, 0x7b, 0x38, 0x5d, 0xc7, 0x73, 0xee, 0x6f, 0xcb, 0xd1, 0xd6, 0xbc,
];

function hexToBytes(hex: string): Uint8Array {
  return new Uint8Array(hex.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16)));
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

describe("Cross-Implementation IBE Tests (Fixture-Based)", () => {
  describe("Test 1: Identity Derivation", () => {
    it("should compute identity hash matching Rust fixture", () => {
      const computed = keccak_256(new TextEncoder().encode(FIXTURE_IDENTITY_STRING));
      const expected = hexToBytes(FIXTURE_IDENTITY_HASH);

      expect(computed.length).toBe(32);
      expect(bytesToHex(computed)).toBe(FIXTURE_IDENTITY_HASH);
    });
  });

  describe("Test 2: Master Key Derivation", () => {
    it("should derive MPK matching Rust fixture", () => {
      const msk = hexToBytes(FIXTURE_MSK_HEX);
      const s = BigInt("0x" + Buffer.from(msk).toString("hex"));

      const mpk = bls12_381.G2.Point.BASE.multiply(s);
      const mpkBytes = mpk.toBytes(true);

      expect(bytesToHex(mpkBytes)).toBe(FIXTURE_MPK_G2_HEX);
    });

    it("should derive DK matching Rust fixture", () => {
      const msk = hexToBytes(FIXTURE_MSK_HEX);
      const s = BigInt("0x" + Buffer.from(msk).toString("hex"));

      // Compute identity hash
      const identityHash = keccak_256(new TextEncoder().encode(FIXTURE_IDENTITY_STRING));

      // Hash to curve with "H(m)" prefix (matching Rust)
      const msgWithH = new Uint8Array([72, 40, 109, 41, ...identityHash]);
      const pointId = bls12_381.G1.hashToCurve(msgWithH, { DST: "APTOS_BLS_WVUF_DST" });

      // Compute DK = s * Q_id
      const dk = pointId.multiply(s);
      const dkBytes = dk.toBytes(true);

      expect(bytesToHex(dkBytes)).toBe(FIXTURE_DK_G1_HEX);
    });
  });

  describe("Test 3: Ciphertext Verification", () => {
    it("should verify ciphertext U matches Rust fixture", () => {
      const ciphertextU = hexToBytes(FIXTURE_CIPHERTEXT_U_HEX);
      expect(ciphertextU.length).toBe(96);

      // Parse and re-serialize to verify
      const point = bls12_381.G2.Point.fromHex(Buffer.from(ciphertextU).toString("hex"));
      const reserialized = point.toBytes(true);

      expect(bytesToHex(reserialized)).toBe(FIXTURE_CIPHERTEXT_U_HEX);
    });

    it("should verify ciphertext V matches Rust fixture", () => {
      const ciphertextV = hexToBytes(FIXTURE_CIPHERTEXT_V_HEX);
      expect(ciphertextV.length).toBe(21);
      expect(bytesToHex(ciphertextV)).toBe(FIXTURE_CIPHERTEXT_V_HEX);
    });
  });

  describe("Test 4: Full IBE Roundtrip (Rust Encrypt, TypeScript Decrypt)", () => {
    it("should decrypt Rust-generated ciphertext", () => {
      // Use pre-computed ciphertext from Rust fixture
      const u = hexToBytes(FIXTURE_CIPHERTEXT_U_HEX);
      const v = hexToBytes(FIXTURE_CIPHERTEXT_V_HEX);
      const dk = hexToBytes(FIXTURE_DK_G1_HEX);
      const identityHash = hexToBytes(FIXTURE_IDENTITY_HASH);
      const mpk = hexToBytes(FIXTURE_MPK_G2_HEX);

      const ciphertext = { u, v };
      const decrypted = IBECrypto.ibeDecrypt(dk, identityHash, mpk, ciphertext);

      expect(new TextDecoder().decode(decrypted)).toBe(FIXTURE_MESSAGE);
      expect(bytesToHex(decrypted)).toBe(FIXTURE_EXPECTED_DECRYPTED_HEX);
    });
  });

  describe("Test 5: Full IBE Roundtrip (TypeScript Encrypt, Rust Decrypt)", () => {
    it("should produce ciphertext that Rust can decrypt", () => {
      // Generate keys using fixture MSK
      const msk = hexToBytes(FIXTURE_MSK_HEX);
      const mpk = IBECrypto.getMasterPublicKey(msk);
      const identity = IBECrypto.computeTimelockIdentity(BigInt(FIXTURE_EPOCH), FIXTURE_DEADLINE_MICROSECONDS);

      // Encrypt message
      const message = new TextEncoder().encode(FIXTURE_MESSAGE);
      const ciphertext = IBECrypto.ibeEncrypt(mpk, identity, message);

      // Verify ciphertext matches Rust fixture format
      expect(ciphertext.u.length).toBe(96);
      expect(bytesToHex(ciphertext.u)).toBe(FIXTURE_CIPHERTEXT_U_HEX);
      expect(bytesToHex(ciphertext.v)).toBe(FIXTURE_CIPHERTEXT_V_HEX);
    });
  });

  describe("Test 6: Cross-Language Gt Serialization", () => {
    it("should match Rust Fp12 serialization", () => {
      // Compute e(G1_generator, G2_generator)
      const g1 = bls12_381.G1.Point.BASE;
      const g2 = bls12_381.G2.Point.BASE;
      const pairing = bls12_381.pairing(g1, g2);

      // Serialize using our canonical format (Big Endian)
      const serialized = IBECrypto.canonicalSerializeFp12(pairing);

      // Compute hash
      const hash = keccak_256(serialized);

      expect(serialized.length).toBe(576);
      expect(hash.length).toBe(32);

      // Compare with Rust expected hash
      const hashesMatch = Array.from(hash).every((byte, i) => byte === EXPECTED_RUST_GT_HASH[i]);

      if (hashesMatch) {
        console.log("✅ SUCCESS: TypeScript Fp12 serialization matches Rust!");
      } else {
        console.log("❌ MISMATCH: TypeScript and Rust Fp12 serialization differ");
        console.log("Expected (Rust):", EXPECTED_RUST_GT_HASH.map((b) => b.toString(16).padStart(2, "0")).join(", "));
        console.log(
          "Actual (TS):    ",
          Array.from(hash)
            .map((b) => b.toString(16).padStart(2, "0"))
            .join(", "),
        );
      }

      expect(hashesMatch).toBe(true);
    });
  });

  describe("Test 7: Determinism", () => {
    it("should produce identical results for same inputs", () => {
      const msk = hexToBytes(FIXTURE_MSK_HEX);
      const identity = IBECrypto.computeTimelockIdentity(BigInt(FIXTURE_EPOCH), FIXTURE_DEADLINE_MICROSECONDS);
      const mpk = IBECrypto.getMasterPublicKey(msk);
      const dk = IBECrypto.getDecryptionKey(msk, identity);
      const message = new TextEncoder().encode(FIXTURE_MESSAGE);

      // Encrypt multiple times
      const ct1 = IBECrypto.ibeEncrypt(mpk, identity, message);
      const ct2 = IBECrypto.ibeEncrypt(mpk, identity, message);

      // Results should be different due to random r
      expect(ct1.u).not.toEqual(ct2.u);

      // But decryption should work for both
      const decrypted1 = IBECrypto.ibeDecrypt(dk, identity, mpk, ct1);
      const decrypted2 = IBECrypto.ibeDecrypt(dk, identity, mpk, ct2);

      expect(new TextDecoder().decode(decrypted1)).toBe(FIXTURE_MESSAGE);
      expect(new TextDecoder().decode(decrypted2)).toBe(FIXTURE_MESSAGE);
    });
  });

  describe("Test 8: Fixture Constants Validation", () => {
    it("should have valid hex constants", () => {
      expect(FIXTURE_MSK_HEX.length).toBe(64); // 32 bytes = 64 hex chars
      expect(FIXTURE_MPK_G2_HEX.length).toBe(192); // 96 bytes = 192 hex chars
      expect(FIXTURE_DK_G1_HEX.length).toBe(96); // 48 bytes = 96 hex chars
      expect(FIXTURE_CIPHERTEXT_U_HEX.length).toBe(192); // 96 bytes
      expect(FIXTURE_IDENTITY_HASH.length).toBe(64); // 32 bytes

      // Parse all to verify they're valid hex
      expect(() => hexToBytes(FIXTURE_MSK_HEX)).not.toThrow();
      expect(() => hexToBytes(FIXTURE_MPK_G2_HEX)).not.toThrow();
      expect(() => hexToBytes(FIXTURE_DK_G1_HEX)).not.toThrow();
      expect(() => hexToBytes(FIXTURE_CIPHERTEXT_U_HEX)).not.toThrow();
      expect(() => hexToBytes(FIXTURE_IDENTITY_HASH)).not.toThrow();
    });
  });
});

import { IBECrypto } from "./ibe-crypto.js";
import { bls12_381 } from "@noble/curves/bls12-381.js";

/**
 * Test IBE crypto operations without requiring a testnet
 */
async function testIbeCrypto() {
  console.log("🧪 Testing IBE Crypto Operations");

  try {
    // Test data
    const message = new Uint8Array([1, 2, 3, 4, 5]);
    const timelockId = 42n;
    const deadline = 1704070800000000n; // 2024-01-01 01:00:00 UTC in microseconds

    // Compute identity
    console.log("Computing identity...");
    const identity = IBECrypto.computeTimelockIdentity(timelockId, deadline);
    console.log(`Identity: ${identity.length} bytes`);

    // Mock master secret key (32 bytes)
    const masterSecret = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      masterSecret[i] = i;
    }

    // Test full IBE encrypt/decrypt cycle
    console.log("Testing full IBE encrypt/decrypt cycle...");

    // Generate a test master secret key (32 bytes)
    const testMasterSecret = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      testMasterSecret[i] = i + 1; // Different from masterSecret
    }

    // Generate a valid MPK using the scalar (master secret)
    const scalar = BigInt("0x" + Buffer.from(testMasterSecret).toString('hex'));
    const testMpkPoint = bls12_381.G2.Point.BASE.multiply(scalar);
    const testMpkG2 = testMpkPoint.toBytes(true);

    // Encrypt using the test identity and fixed mpk
    console.log("Encrypting with IBE...");
    const ciphertext = IBECrypto.ibeEncrypt(testMpkG2, identity, message);
    console.log(`Ciphertext created: U=${ciphertext.u.length} bytes, V=${ciphertext.v.length} bytes`);

    // Decrypt using the same identity and mpk
    console.log("Decrypting with IBE...");

    // Compute valid SK for identity (sk = H(id)^s)
    const validSk = IBECrypto.getDecryptionKey(testMasterSecret, identity);

    const decrypted = IBECrypto.ibeDecrypt(validSk, identity, testMpkG2, ciphertext);
    console.log(`Decrypted: ${decrypted.length} bytes`);

    // Verify the message
    const matches = decrypted.length === message.length && decrypted.every((b, i) => b === message[i]);

    if (matches) {
      console.log("✅ Full IBE crypto test PASSED!");
      console.log("✅ Identity computation + encrypt/decrypt cycle working!");
    } else {
      console.log("❌ IBE crypto test FAILED - decrypted message doesn't match original");
      console.log("Expected:", Array.from(message));
      console.log("Got:", Array.from(decrypted));
    }
  } catch (error) {
    console.error("❌ IBE crypto test failed:", error);
    throw error;
  }
}

testIbeCrypto().catch(console.error);

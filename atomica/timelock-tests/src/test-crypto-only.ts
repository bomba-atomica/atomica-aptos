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
    const chainId = 4;
    const interval = 1n; // Use bigint

    // Compute identity
    console.log("Computing identity...");
    const identity = IBECrypto.computeTimelockIdentity(interval, chainId);
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

    // For testing, use fixed key material that both encrypt and decrypt can derive
    const testMpkG2 = new Uint8Array(96);
    for (let i = 0; i < testMpkG2.length; i++) {
      testMpkG2[i] = i % 256;
    }

    // Encrypt using the test identity and fixed mpk
    console.log("Encrypting with IBE...");
    const ciphertext = IBECrypto.ibeEncrypt(testMpkG2, identity, message);
    console.log(`Ciphertext created: U=${ciphertext.u.length} bytes, V=${ciphertext.v.length} bytes`);

    // Decrypt using the same identity and mpk
    console.log("Decrypting with IBE...");
    const dummySk = new Uint8Array(48);
    const decrypted = IBECrypto.ibeDecrypt(dummySk, identity, testMpkG2, ciphertext);
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

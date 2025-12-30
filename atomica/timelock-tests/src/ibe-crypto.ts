import { bls12_381 } from "@noble/curves/bls12-381.js";
import { keccak_256 } from "@noble/hashes/sha3.js";

/**
 * IBE (Identity-Based Encryption) cryptographic operations using BLS12-381
 */

export interface Ciphertext {
  u: Uint8Array; // 96 bytes - G2 point (U component)
  v: Uint8Array; // encrypted message (V component)
}

export class IBECrypto {
  /**
   * Compute timelock identity from interval and chain ID
   * Identity = H(interval || chain_id || "atomica_timelock")
   */
  static computeTimelockIdentity(interval: bigint, chainId: number): Uint8Array {
    const hasher = keccak_256.create();

    // Add interval as little-endian bytes
    const intervalBytes = new Uint8Array(8);
    const view = new DataView(intervalBytes.buffer);
    view.setBigUint64(0, interval, true);
    hasher.update(intervalBytes);

    // Add chain ID
    hasher.update(new Uint8Array([chainId]));

    // Add domain separator
    hasher.update(new TextEncoder().encode("atomica_timelock"));

    return hasher.digest();
  }

  /**
   * Encrypt a message using IBE with the given public key (G2 point) and identity
   * Ciphertext = (U = r * G2, V = M ⊕ H(e(identity, mpk)))
   */
  static ibeEncrypt(mpkG2: Uint8Array, identity: Uint8Array, message: Uint8Array): Ciphertext {
    // For testing, use a deterministic key based on identity and mpk
    const keyMaterial = new Uint8Array(identity.length + mpkG2.length);
    keyMaterial.set(identity);
    keyMaterial.set(mpkG2, identity.length);

    const symmetricKey = keccak_256(keyMaterial).slice(0, 32);
    console.log("Encrypt key:", Array.from(symmetricKey.slice(0, 4)), "...");

    // Use a fixed U component for testing (this would be r*G2 in real implementation)
    const u = new Uint8Array(96); // G2 compressed point size
    u[0] = 1; // Just set first byte for testing

    // XOR encrypt message
    return {
      u,
      v: this.xorBytes(message, symmetricKey),
    };
  }

  /**
   * Decrypt a ciphertext using IBE with the given secret key (G1 point)
   * Message = V ⊕ H2(e(sk, U))
   */
  static ibeDecrypt(skG1: Uint8Array, identity: Uint8Array, mpkG2: Uint8Array, ciphertext: Ciphertext): Uint8Array {
    // For testing, use the same key derivation as encrypt
    const keyMaterial = new Uint8Array(identity.length + mpkG2.length);
    keyMaterial.set(identity);
    keyMaterial.set(mpkG2, identity.length);

    const symmetricKey = keccak_256(keyMaterial).slice(0, 32);
    console.log("Decrypt key:", Array.from(symmetricKey.slice(0, 4)), "...");

    // XOR decrypt message
    return this.xorBytes(ciphertext.v, symmetricKey);
  }

  /**
   * Deserialize a G1 point from compressed bytes (48 bytes)
   */
  static deserializeG1(bytes: Uint8Array): bls12_381.G1.Point {
    if (bytes.length !== 48) {
      throw new Error(`Invalid G1 compressed length: expected 48, got ${bytes.length}`);
    }
    return bls12_381.G1.Point.fromHex(bytes);
  }

  static deserializeG2(bytes: Uint8Array): bls12_381.G2.Point {
    if (bytes.length !== 96) {
      throw new Error(`Invalid G2 compressed length: expected 96, got ${bytes.length}`);
    }
    return bls12_381.G2.Point.fromHex(bytes);
  }

  /**
   * Extract G2 master public key from DKG transcript
   * This is a simplified implementation - real implementation would parse BCS
   */
  static extractG2FromTranscript(transcriptBytes: Uint8Array): Uint8Array {
    // TODO: Implement proper BCS parsing of transcript structure
    // For now, assume the transcript contains the MPK as the first field
    if (transcriptBytes.length < 96) {
      throw new Error(`Transcript too short for G2 extraction: ${transcriptBytes.length} bytes`);
    }
    return transcriptBytes.slice(0, 96);
  }

  /**
   * XOR two byte arrays for symmetric encryption
   */
  private static xorBytes(a: Uint8Array, b: Uint8Array): Uint8Array {
    const result = new Uint8Array(a.length);
    for (let i = 0; i < a.length; i++) {
      result[i] = a[i] ^ b[i % b.length];
    }
    return result;
  }
}

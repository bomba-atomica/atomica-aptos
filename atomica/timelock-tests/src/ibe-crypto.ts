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

    // Add interval as little-endian bytes (8 bytes)
    const intervalBytes = new Uint8Array(8);
    const view = new DataView(intervalBytes.buffer);
    view.setBigUint64(0, interval, true);
    hasher.update(intervalBytes);

    // Add chain ID (1 byte - assuming < 256 for testnet)
    hasher.update(new Uint8Array([chainId]));

    // Add domain separator
    hasher.update(new TextEncoder().encode("atomica_timelock"));

    return hasher.digest();
  }

  /**
   * Encrypt a message using IBE with the given public key (G2 point) and identity
   * 
   * Identity-Based Encryption (Boneh-Franklin / Sakai-Kasahara style with pairings):
   * 1. H_id = MapToG1(identity)
   * 2. r = random scalar
   * 3. U = r * G2_generator (randomness commitment)
   * 4. g_id = e(H_id, mpk) (shared secret base)
   * 5. V = message XOR H(g_id^r)
   * 6. Ciphertext = (U, V)
   */
  static ibeEncrypt(mpkG2: Uint8Array, identity: Uint8Array, message: Uint8Array): Ciphertext {
    // 1. Map identity to G1
    const pointId = bls12_381.G1.hashToCurve(identity);

    // 2. Parse MPK
    const mpkPoint = bls12_381.G2.Point.fromHex(Buffer.from(mpkG2).toString('hex'));

    // 3. Generate random r
    const r = bls12_381.utils.randomSecretKey();

    // 4. U = r * G2_generator
    const uPoint = bls12_381.G2.Point.BASE.multiply(BigInt("0x" + Buffer.from(r).toString('hex')));
    const u = uPoint.toBytes(true);

    // 5. Symmetric Key Generation
    // We compute pairing(H_id * r, mpk) => e(H_id, mpk)^r
    const pointIdTimesR = pointId.multiply(BigInt("0x" + Buffer.from(r).toString('hex')));
    const sharedSecret = bls12_381.pairing(pointIdTimesR, mpkPoint);

    // Convert Fp12 shared secret to bytes for hashing
    // @ts-ignore
    const sharedSecretBytes = bls12_381.fields.Fp12.toBytes(sharedSecret);

    const symmetricKey = keccak_256(sharedSecretBytes).slice(0, 32);

    return {
      u,
      v: this.xorBytes(message, symmetricKey),
    };
  }

  /**
   * Decrypt a ciphertext using IBE with the given secret key (G1 point)
   * 
   * Secret Key sk = H_id^s (where s is master secret)
   * U = r * G2
   * Shared Secret = e(sk, U) = e(H_id^s, r*G2) = e(H_id, G2)^(s*r)
   * This matches encryption: e(H_id, mpk)^r = e(H_id, s*G2)^r = e(H_id, G2)^(s*r)
   */
  static ibeDecrypt(skG1: Uint8Array, identity: Uint8Array, mpkG2: Uint8Array, ciphertext: Ciphertext): Uint8Array {
    // 1. Parse U and SK
    const uPoint = bls12_381.G2.Point.fromHex(Buffer.from(ciphertext.u).toString('hex'));
    const skPoint = bls12_381.G1.Point.fromHex(Buffer.from(skG1).toString('hex'));

    // 2. Compute Pairing e(sk, U)
    const sharedSecret = bls12_381.pairing(skPoint, uPoint);

    // 3. Derive symmetric key
    // @ts-ignore
    const sharedSecretBytes = bls12_381.fields.Fp12.toBytes(sharedSecret);
    const symmetricKey = keccak_256(sharedSecretBytes).slice(0, 32);

    // 4. Decrypt
    return this.xorBytes(ciphertext.v, symmetricKey);
  }

  /**
   * Deserialize a G1 point from compressed bytes (48 bytes)
   */
  static deserializeG1(bytes: Uint8Array) {
    if (bytes.length !== 48) {
      if (bytes.length !== 96) {
        throw new Error(`Invalid G1 length: expected 48 (compressed), got ${bytes.length}`);
      }
    }
    return bls12_381.G1.Point.fromHex(Buffer.from(bytes).toString('hex'));
  }

  static deserializeG2(bytes: Uint8Array) {
    if (bytes.length !== 96) {
      if (bytes.length !== 192) {
        throw new Error(`Invalid G2 length: expected 96 (compressed), got ${bytes.length}`);
      }
    }
    return bls12_381.G2.Point.fromHex(Buffer.from(bytes).toString('hex'));
  }

  /**
   * Extract G2 master public key from DKG transcript
   */
  static extractG2FromTranscript(transcriptBytes: Uint8Array): Uint8Array {
    // Simplification for MVP: assume first 96 bytes are the MPK
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

  // --- DKG Simulation Helpers ---

  /**
   * Generate a random master secret (scalar)
   */
  static generateMasterSecret(): Uint8Array {
    return bls12_381.utils.randomSecretKey();
  }

  /**
   * Get Master Public Key (G2) from Master Secret
   * MPK = s * G2_generator
   */
  static getMasterPublicKey(msk: Uint8Array): Uint8Array {
    const s = BigInt("0x" + Buffer.from(msk).toString("hex"));
    const mpk = bls12_381.G2.Point.BASE.multiply(s);
    return mpk.toBytes(true);
  }

  /**
   * Get Decryption Key (G1) for an identity
   * DK = s * H(id)
   */
  static getDecryptionKey(msk: Uint8Array, identity: Uint8Array): Uint8Array {
    const s = BigInt("0x" + Buffer.from(msk).toString("hex"));
    const pointId = bls12_381.G1.hashToCurve(identity);
    const dk = pointId.multiply(s);
    return dk.toBytes(true);
  }

  /**
   * Generate additive shares for a G1 secret
   * Returns n shares such that sum(shares) = secret
   * WARNING: Simple additive sharing, assumes contract just sums shares.
   */
  static generateAdditiveShares(secretG1Bytes: Uint8Array, n: number): Uint8Array[] {
    const secret = bls12_381.G1.Point.fromHex(Buffer.from(secretG1Bytes).toString('hex'));
    const shares: typeof secret[] = [];
    let currentSum = bls12_381.G1.Point.ZERO;

    // Generate n-1 random shares
    for (let i = 0; i < n - 1; i++) {
      const r = bls12_381.utils.randomSecretKey();
      const scalar = BigInt("0x" + Buffer.from(r).toString("hex"));
      const share = bls12_381.G1.Point.BASE.multiply(scalar);
      shares.push(share);
      currentSum = currentSum.add(share);
    }

    // Last share = secret - sum(others)
    const lastShare = secret.subtract(currentSum);
    shares.push(lastShare);

    return shares.map(p => p.toBytes(true));
  }
}

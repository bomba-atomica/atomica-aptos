import { bls12_381 } from "@noble/curves/bls12-381.js";
import { keccak_256 } from "@noble/hashes/sha3.js";

/**
 * Minimal BCS Reader helper
 */
class BCSReader {
  private view: DataView;
  private offset: number = 0;

  constructor(data: Uint8Array) {
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  }

  readU8(): number {
    const val = this.view.getUint8(this.offset);
    this.offset += 1;
    return val;
  }

  readU64(): bigint {
    const val = this.view.getBigUint64(this.offset, true); // Little endian
    this.offset += 8;
    return val;
  }

  readBytes(count: number): Uint8Array {
    const val = new Uint8Array(this.view.buffer, this.view.byteOffset + this.offset, count);
    this.offset += count;
    return new Uint8Array(val); // copy to avoid buffer issues
  }

  // ULEB128 for vector lengths
  readUleb128(): number {
    let result = 0;
    let shift = 0;
    while (true) {
      const byte = this.readU8();
      result |= (byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) break;
      shift += 7;
    }
    return result;
  }

  readBytesVector(): Uint8Array {
    const len = this.readUleb128();
    return this.readBytes(len);
  }
}

/**
 * IBE (Identity-Based Encryption) cryptographic operations using BLS12-381
 */

export interface Ciphertext {
  u: Uint8Array; // 96 bytes - G2 point (U component)
  v: Uint8Array; // encrypted message (V component)
}

export class IBECrypto {
  /**
   * Compute timelock identity from timelock ID and deadline.
   * 
   * Format: Keccak256("timelock_id:{id}:deadline_timestamp_microseconds:{deadline}")
   * 
   * This is an **application-agnostic** identity format. It contains no auction,
   * bid, or other application-specific semantics.
   * 
   * @param timelockId - Unique identifier for this timelock
   * @param deadlineTimestampMicroseconds - Unix epoch timestamp in MICROSECONDS when decryption becomes available
   * @returns 32-byte Keccak256 hash
   */
  static computeTimelockIdentity(timelockId: bigint, deadlineTimestampMicroseconds: bigint): Uint8Array {
    // Construct canonical identity using human-readable string format
    // Must match Rust implementation exactly
    const identityString = `timelock_id:${timelockId}:deadline_timestamp_microseconds:${deadlineTimestampMicroseconds}`;
    return keccak_256(new TextEncoder().encode(identityString));
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
  // DST matches Rust BLS_WVUF_DST
  static readonly DST = "APTOS_BLS_WVUF_DST";

  /**
   * Encrypts a message using IBE.
   * 
   * @param mpkG2 Master Public Key (G2 point)
   * @param identity Identity bytes
   * @param message Message to encrypt
   * @returns Ciphertext
   */
  static ibeEncrypt(mpkG2: Uint8Array, identity: Uint8Array, message: Uint8Array): Ciphertext {
    // 1. Map identity to G1
    // Timelock IBE uses identity = H(interval...), no "H(m)" augmentation.
    const pointId = bls12_381.G1.hashToCurve(identity, { DST: IBECrypto.DST });

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
    const sharedSecretBytes = IBECrypto.canonicalSerializeFp12(sharedSecret);

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
    const sharedSecretBytes = IBECrypto.canonicalSerializeFp12(sharedSecret);
    const symmetricKey = keccak_256(sharedSecretBytes).slice(0, 32);

    // 4. Decrypt
    return this.xorBytes(ciphertext.v, symmetricKey);
  }

  /**
   * Deserialize a G1 point from compressed bytes (48 bytes)
   * Handles both "compressed" (0x80|0x00 flag) and raw formats slightly more robustly if needed.
   * For BLS12-381, compressed G1 is 48 bytes.
   */
  static deserializeG1(bytes: Uint8Array) {
    // Ensure 48 bytes
    if (bytes.length !== 48) throw new Error("G1 point must be 48 bytes");
    return bls12_381.G1.Point.fromHex(Buffer.from(bytes).toString('hex'));
  }

  static deserializeG2(bytes: Uint8Array) {
    if (bytes.length !== 96) throw new Error("G2 point must be 96 bytes");
    return bls12_381.G2.Point.fromHex(Buffer.from(bytes).toString('hex'));
  }

  /**
   * Extract G2 master public key from DKG transcript (BCS serialized)
   * 
   * Rust Structure:
   * struct DKGTranscript {
   *     metadata: DKGTranscriptMetadata, // epoch(u64), author(32 bytes)
   *     transcript_bytes: vector<u8>,    // BCS bytes of Transcripts
   * }
   * 
   * struct Transcripts {
   *     main: WeightedTranscript,
   *     fast: Option<WeightedTranscript>,
   * }
   * 
   * struct WeightedTranscript {
   *     soks: Vec<SoK>,
   *     R: Vec<G1>,
   *     R_hat: Vec<G2>,
   *     V: Vec<G1>,
   *     V_hat: Vec<G2>, // <--- Target: Last element of this vector is the MPK
   *     C: Vec<G1>,
   * }
   */
  static extractG2FromTranscript(transcriptBytes: Uint8Array): Uint8Array {
    const reader = new BCSReader(transcriptBytes);

    // 1. DKGTranscript
    // metadata.epoch (u64)
    reader.readU64();
    // metadata.author (32 bytes)
    reader.readBytes(32);

    // transcript_bytes (vector<u8>)
    const innerBytes = reader.readBytesVector();

    // 2. Transcripts
    const innerReader = new BCSReader(innerBytes);

    // main: WeightedTranscript
    //   soks: Vec<SoK>
    const numSoks = innerReader.readUleb128();
    for (let i = 0; i < numSoks; i++) {
      // SoK: (Player, G1, Signature, PoK)
      // Player: id (usize -> u64 in BCS for Aptos?)
      innerReader.readU64(); // id
      innerReader.readBytes(48); // comm (G1)
      innerReader.readBytes(96); // sig (Signature - G2 for min-pk)
      // PoK: (G1, Scalar)
      innerReader.readBytes(48); // G1
      innerReader.readBytes(32); // Scalar
    }

    //   R: Vec<G1>
    const numR = innerReader.readUleb128();
    for (let i = 0; i < numR; i++) innerReader.readBytes(48);

    //   R_hat: Vec<G2>
    const numRhat = innerReader.readUleb128();
    for (let i = 0; i < numRhat; i++) innerReader.readBytes(96);

    //   V: Vec<G1>
    const numV = innerReader.readUleb128();
    for (let i = 0; i < numV; i++) innerReader.readBytes(48);

    //   V_hat: Vec<G2>
    const numVhat = innerReader.readUleb128();
    if (numVhat === 0) {
      throw new Error("Invalid Transcript: V_hat is empty, cannot extract MPK");
    }

    // We need the LAST element of V_hat.
    // Skip the first N-1 elements
    for (let i = 0; i < numVhat - 1; i++) {
      innerReader.readBytes(96);
    }

    // Read the last one (MPK)
    const mpk = innerReader.readBytes(96);

    return mpk;
  }

  /**
   * Serialize Fp12 element to bytes in Little Endian format to match Rust/Arkworks
   * Order: c0.c0.c0, c0.c0.c1, c0.c1.c0 ... c1.c2.c1
   * Each Fp element is 48 bytes, Little Endian.
   */
  static canonicalSerializeFp12(fp12: any): Uint8Array {
    const result = new Uint8Array(576); // 12 * 48
    let offset = 0;

    const coeffs = [
      fp12.c0.c0.c0, fp12.c0.c0.c1, // Fp2 c0
      fp12.c0.c1.c0, fp12.c0.c1.c1, // Fp2 c1
      fp12.c0.c2.c0, fp12.c0.c2.c1, // Fp2 c2
      fp12.c1.c0.c0, fp12.c1.c0.c1, // Fp2 c0 (of c1)
      fp12.c1.c1.c0, fp12.c1.c1.c1, // Fp2 c1 (of c1)
      fp12.c1.c2.c0, fp12.c1.c2.c1, // Fp2 c2 (of c1)
    ];

    for (const val of coeffs) {
      let hex = val.toString(16);
      if (hex.length % 2 !== 0) hex = '0' + hex;
      const padding = 96 - hex.length; // 48 bytes = 96 hex chars
      if (padding > 0) hex = '0'.repeat(padding) + hex;

      const buffer = Buffer.from(hex, 'hex');
      // Reverse for Little Endian
      result.set(buffer.reverse(), offset);
      offset += 48;
    }

    return result;
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
    const pointId = bls12_381.G1.hashToCurve(identity, { DST: IBECrypto.DST });
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

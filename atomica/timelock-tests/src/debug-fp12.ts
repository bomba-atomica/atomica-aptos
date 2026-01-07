
import { bls12_381 } from "@noble/curves/bls12-381.js";

const g1 = bls12_381.G1.Point.BASE;
const g2 = bls12_381.G2.Point.BASE;
const pairing = bls12_381.pairing(g1, g2);

console.log("Structure of Fp12 element:");
console.log(JSON.stringify(pairing, (key, value) =>
    typeof value === 'bigint' ? value.toString() + 'n' : value
    , 2));

// Check if we can access coefficients directly
// bls12-381 Fp12 is typically a quadratic extension of Fp6 (w^2 - v = 0) or similar.
// Let's see property names.
import { keccak_256 } from "@noble/hashes/sha3.js";

// Helper to serialize Fp to bytes
function fpToBytes(val: bigint, littleEndian: boolean): Uint8Array {
    let hex = val.toString(16);
    if (hex.length % 2 !== 0) hex = '0' + hex;
    const padding = 96 - hex.length;
    if (padding > 0) hex = '0'.repeat(padding) + hex;
    const buffer = Buffer.from(hex, 'hex');
    if (littleEndian) return buffer.reverse();
    return buffer;
}

const coeffs = [
    pairing.c0.c0.c0, pairing.c0.c0.c1, // Fp2 c0
    pairing.c0.c1.c0, pairing.c0.c1.c1, // Fp2 c1
    pairing.c0.c2.c0, pairing.c0.c2.c1, // Fp2 c2
    pairing.c1.c0.c0, pairing.c1.c0.c1,
    pairing.c1.c1.c0, pairing.c1.c1.c1,
    pairing.c1.c2.c0, pairing.c1.c2.c1
];

// 1. Standard Order, Big Endian (Current TS)
const standardBE = new Uint8Array(576);
let offset = 0;
for (const c of coeffs) {
    standardBE.set(fpToBytes(c, false), offset);
    offset += 48;
}

// 2. Standard Order, Little Endian
const standardLE = new Uint8Array(576);
offset = 0;
for (const c of coeffs) {
    standardLE.set(fpToBytes(c, true), offset);
    offset += 48;
}

// 3. Fp12 Swapped (C1, C0), Big Endian
const swapedFp12BE = new Uint8Array(576);
const C0_coeffs = coeffs.slice(0, 6);
const C1_coeffs = coeffs.slice(6, 12);
offset = 0;
for (const c of C1_coeffs) { swapedFp12BE.set(fpToBytes(c, false), offset); offset += 48; }
for (const c of C0_coeffs) { swapedFp12BE.set(fpToBytes(c, false), offset); offset += 48; }

// Compute Hashes
function toHex(bytes: Uint8Array): string {
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

console.log("Rust Target Hash start: 8cf9fb0d...");

const h1 = keccak_256(standardBE);
console.log(`Standard BE: ${toHex(h1).slice(0, 32)}...`);

const h2 = keccak_256(standardLE);
console.log(`Standard LE: ${toHex(h2).slice(0, 32)}...`);

const h3 = keccak_256(swapedFp12BE);
console.log(`Swapped Fp12 BE: ${toHex(h3).slice(0, 32)}...`);

// 4. Arkworks Legacy (Fp2 swapped? c1, c0)
// Some libraries do c1 then c0 for Fp2.
// Let's try that.
const fp2SwappedBE = new Uint8Array(576);
offset = 0;
for (let i = 0; i < 12; i += 2) {
    fp2SwappedBE.set(fpToBytes(coeffs[i + 1], false), offset); offset += 48;
    fp2SwappedBE.set(fpToBytes(coeffs[i], false), offset); offset += 48;
}
const h4 = keccak_256(fp2SwappedBE);
console.log(`Swapped Fp2 BE: ${toHex(h4).slice(0, 32)}...`);

// Helper to serialize Fp to hex (Big Endian, 48 bytes)
function fpToHex(val: bigint): string {
    let hex = val.toString(16);
    if (hex.length % 2 !== 0) hex = '0' + hex;
    const padding = 96 - hex.length;
    if (padding > 0) hex = '0'.repeat(padding) + hex;
    return hex;
}

const c0_c0_c0 = fpToHex(pairing.c0.c0.c0);
const c0_c0_c1 = fpToHex(pairing.c0.c0.c1);
const c0_c1_c0 = fpToHex(pairing.c0.c1.c0);
const c0_c1_c1 = fpToHex(pairing.c0.c1.c1);
const c0_c2_c0 = fpToHex(pairing.c0.c2.c0);
const c0_c2_c1 = fpToHex(pairing.c0.c2.c1);

const c1_c0_c0 = fpToHex(pairing.c1.c0.c0);
const c1_c0_c1 = fpToHex(pairing.c1.c0.c1);
const c1_c1_c0 = fpToHex(pairing.c1.c1.c0);
const c1_c1_c1 = fpToHex(pairing.c1.c1.c1);
const c1_c2_c0 = fpToHex(pairing.c1.c2.c0);
const c1_c2_c1 = fpToHex(pairing.c1.c2.c1);

console.log(`c0.c0.c0: ${c0_c0_c0}`);
console.log(`c0.c0.c1: ${c0_c0_c1}`);
console.log(`c0.c1.c0: ${c0_c1_c0}`);
console.log(`c0.c1.c1: ${c0_c1_c1}`);
console.log(`c0.c2.c0: ${c0_c2_c0}`);
console.log(`c0.c2.c1: ${c0_c2_c1}`);

console.log(`c1.c0.c0: ${c1_c0_c0}`);
console.log(`c1.c0.c1: ${c1_c0_c1}`);
console.log(`c1.c1.c0: ${c1_c1_c0}`);
console.log(`c1.c1.c1: ${c1_c1_c1}`);
console.log(`c1.c2.c0: ${c1_c2_c0}`);
console.log(`c1.c2.c1: ${c1_c2_c1}`);


// Test Bilinear Property
console.log("\nTesting Bilinear Property:");
const r = bls12_381.utils.randomSecretKey();
const s = bls12_381.utils.randomSecretKey();
const rBig = BigInt("0x" + Buffer.from(r).toString('hex'));
const sBig = BigInt("0x" + Buffer.from(s).toString('hex'));

const H = bls12_381.G1.hashToCurve(new Uint8Array([1, 2, 3]), { DST: "APTOS_BLS_WVUF_DST" });
const G2 = bls12_381.G2.Point.BASE;

// Left: e(r*H, s*G2)
const rH = H.multiply(rBig);
const sG2 = G2.multiply(sBig);
const left = bls12_381.pairing(rH, sG2);

// Right: e(s*H, r*G2)
const sH = H.multiply(sBig);
const rG2 = G2.multiply(rBig);
const right = bls12_381.pairing(sH, rG2);


// console.log("e(r*H, s*G2) == e(s*H, r*G2)?", bls12_381.fields.Fp12.eq(left, right));


// Also check serialization
const leftHex = Buffer.from(fpToBytes(left.c0.c0.c0, false)).toString('hex');
const rightHex = Buffer.from(fpToBytes(right.c0.c0.c0, false)).toString('hex');
console.log("Left c0.c0.c0:", leftHex);
console.log("Right c0.c0.c0:", rightHex);


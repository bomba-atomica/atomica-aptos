import { IBECrypto } from "./ibe-crypto.js";
import { bls12_381 } from "@noble/curves/bls12-381.js";

/**
 * Test the DKG Transcript Parsers logic by constructing a synthetic BCS payload.
 */
async function testTranscriptParser() {
    console.log("🧪 Testing DKG Transcript Parsing Logic");

    // 1. Construct Mock DKG Transcript
    // Structure:
    // DKGTranscript {
    //   metadata: { epoch: u64, author: 32 bytes }
    //   transcript_bytes: vector<u8> (contains Transcripts)
    // }

    // Transcripts {
    //   main: WeightedTranscript {
    //     soks: Vec<SoK> (len 0)
    //     R: Vec<G1> (len 0)
    //     R_hat: Vec<G2> (len 0)
    //     V: Vec<G1> (len 0)
    //     V_hat: Vec<G2> (len 2 - First point dummy, Last point TARGET MPK)
    //     C: Vec<G1> (len 0)
    //   }
    //   fast: Option<WeightedTranscript> (0 = None)
    // }

    // Utils for BCS
    const u64ToBytes = (val: bigint) => {
        const b = new Uint8Array(8);
        const v = new DataView(b.buffer);
        v.setBigUint64(0, val, true);
        return b;
    }

    const uleb128 = (val: number) => {
        const res = [];
        while (true) {
            let byte = val & 0x7f;
            val >>= 7;
            if (val !== 0) {
                byte |= 0x80;
                res.push(byte);
            } else {
                res.push(byte);
                break;
            }
        }
        return new Uint8Array(res);
    }

    // --- Construct Inner Transcript (Transcripts) ---
    const chunks: Uint8Array[] = [];

    // soks len 0
    chunks.push(uleb128(0));
    // R len 0
    chunks.push(uleb128(0));
    // R_hat len 0
    chunks.push(uleb128(0));
    // V len 0
    chunks.push(uleb128(0));

    // V_hat len 2
    chunks.push(uleb128(2));

    // V_hat[0]: Dummy G2 (96 bytes)
    const dummyG2 = new Uint8Array(96).fill(0xAA);
    chunks.push(dummyG2);

    // V_hat[1]: TARGET MPK (96 bytes)
    const targetMpkBytes = new Uint8Array(96).fill(0xBB);
    targetMpkBytes[0] = 0xBE;
    targetMpkBytes[95] = 0xEF;
    chunks.push(targetMpkBytes);

    // C len 0
    chunks.push(uleb128(0));

    // Fast path Option (0 = None)
    chunks.push(new Uint8Array([0]));

    // Combine Inner Bytes
    const innerBytes = new Uint8Array(chunks.reduce((acc, curr) => acc + curr.length, 0));
    let offset = 0;
    for (const c of chunks) {
        innerBytes.set(c, offset);
        offset += c.length;
    }

    // --- Construct Outer DKGTranscript ---
    const outerChunks: Uint8Array[] = [];

    // Metadata: epoch (u64)
    outerChunks.push(u64ToBytes(10n));
    // Metadata: author (32 bytes)
    outerChunks.push(new Uint8Array(32).fill(1));

    // Transcript Bytes: Vector<u8> -> ULEB128 len + bytes
    outerChunks.push(uleb128(innerBytes.length));
    outerChunks.push(innerBytes);

    const dkgTranscript = new Uint8Array(outerChunks.reduce((acc, curr) => acc + curr.length, 0));
    let outerOffset = 0;
    for (const c of outerChunks) {
        dkgTranscript.set(c, outerOffset);
        outerOffset += c.length;
    }

    console.log(`Constructed synthetic DKG Transcript: ${dkgTranscript.length} bytes`);

    // 2. Parse it
    try {
        console.log("Extracting MPK...");
        const extractedMpk = IBECrypto.extractG2FromTranscript(dkgTranscript);

        // 3. Verify
        let match = true;
        if (extractedMpk.length !== 96) {
            console.error(`MPK length mismatch: expected 96, got ${extractedMpk.length}`);
            match = false;
        }

        for (let i = 0; i < 96; i++) {
            if (extractedMpk[i] !== targetMpkBytes[i]) {
                console.error(`Byte mismatch at ${i}: expected ${targetMpkBytes[i]}, got ${extractedMpk[i]}`);
                match = false;
                break;
            }
        }

        if (match) {
            console.log("✅ MPK extracted successfully and matches target!");
        } else {
            console.error("❌ MPK extraction failed validation");
            process.exit(1);
        }

    } catch (error) {
        console.error("❌ Parsing failed with exception:", error);
        process.exit(1);
    }
}

testTranscriptParser().catch(console.error);

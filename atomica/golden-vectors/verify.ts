
import { IBECrypto, Ciphertext } from "../timelock-tests/src/ibe-crypto.js";
import { bls12_381 } from "@noble/curves/bls12-381.js";
import * as fs from "fs";
import * as path from "path";

async function verifyGoldenVectors() {
    console.log("🔍 Verifying Rust Golden Vectors in TypeScript...");

    const jsonPath = path.join(process.cwd(), "atomica/golden-vectors/ibe_fixtures.json");
    if (!fs.existsSync(jsonPath)) {
        console.error(`❌ Fixtures file not found at ${jsonPath}`);
        process.exit(1);
    }

    const fixture = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
    console.log(`Loaded fixture: ${fixture.description} (${fixture.timestamp})`);

    // Helper: Hex to Bytes
    const h2b = (hex: string) => Uint8Array.from(Buffer.from(hex, 'hex'));

    let allPassed = true;

    // 1. Verify Identity Computation
    console.log("\n1. Testing Identity Computation...");
    const interval = BigInt(fixture.parameters.interval);
    const chainId = fixture.parameters.chain_id;
    const computedIdentity = IBECrypto.computeTimelockIdentity(interval, chainId);
    const expectedIdentity = h2b(fixture.keys.identity_hash_hex);

    if (Buffer.compare(computedIdentity, expectedIdentity) === 0) {
        console.log("✅ Identity computation matches.");
    } else {
        console.log("❌ Identity mismatch!");
        console.log(`   Expected: ${fixture.keys.identity_hash_hex}`);
        console.log(`   Got:      ${Buffer.from(computedIdentity).toString('hex')}`);
        allPassed = false;
    }

    // 2. Verify Key Deserialization (G1/G2)
    console.log("\n2. Testing Key Deserialization...");
    const mpkBytes = h2b(fixture.keys.mpk_g2_hex);
    const dkBytes = h2b(fixture.keys.decryption_key_g1_hex);

    try {
        // Just attempting to deserialize checks if points are valid/on-curve
        IBECrypto.deserializeG2(mpkBytes);
        console.log("✅ MPK (G2) deserialized successfully.");
    } catch (e) {
        console.log("❌ MPK deserialization failed:", e);
        allPassed = false;
    }

    try {
        IBECrypto.deserializeG1(dkBytes);
        console.log("✅ DK (G1) deserialized successfully.");
    } catch (e) {
        console.log("❌ DK deserialization failed:", e);
        allPassed = false;
    }

    // 2.5 Verify Generator Consistency (G2)
    console.log("\n2.5 Testing Generator Consistency...");
    const mskHexLE = fixture.keys.msk_hex;
    // Rust msk_hex is Little Endian (scalar.to_bytes_le())
    // BigInt expects Big Endian hex.
    const mskHexBE = mskHexLE.match(/../g).reverse().join("");
    const msk = BigInt("0x" + mskHexBE);

    const derivedMpk = bls12_381.G2.Point.BASE.multiply(msk);
    const derivedMpkBytes = derivedMpk.toBytes(true); // Compressed

    if (Buffer.compare(derivedMpkBytes, mpkBytes) === 0) {
        console.log("✅ G2 Generators match (G2_TS * msk == MPK_Rust)");
    } else {
        console.log("❌ G2 Generator Mismatch!");
        console.log(`   Derived MPK from MSK: ${Buffer.from(derivedMpkBytes).toString('hex')}`);
        console.log(`   Actual MPK from Rust: ${Buffer.from(mpkBytes).toString('hex')}`);
        // This confirms if G2_Base is different
        allPassed = false;
    }

    // 3. Verify Decryption from Rust Ciphertext
    console.log("\n3. Testing Decryption of Rust Ciphertext...");
    const ciphertext: Ciphertext = {
        u: h2b(fixture.ciphertext.u_g2_hex),
        v: h2b(fixture.ciphertext.v_bytes_hex),
    };

    // We pass the raw bytes from the fixture.
    try {
        const decrypted = IBECrypto.ibeDecrypt(
            dkBytes,
            computedIdentity,
            mpkBytes,
            ciphertext
        );
        const decryptedHex = Buffer.from(decrypted).toString('hex');
        const decryptedStr = new TextDecoder().decode(decrypted);

        if (decryptedHex === fixture.verification.decrypted_hex) {
            console.log(`✅ Decryption successful!`);
            console.log(`   Message: "${decryptedStr}"`);
        } else {
            console.log("❌ Decryption mismatch!");
            console.log(`   Expected Hex: ${fixture.verification.decrypted_hex}`);
            console.log(`   Got Hex:      ${decryptedHex}`);
            allPassed = false;
        }
    } catch (e) {
        console.log("❌ Decryption threw exception:", e);
        allPassed = false;
    }

    // 4. Verify Encryption (Optional - can't check ciphertext equality due to randomness, 
    //    but can check if we can decrypt our OWN ciphertext with Rust keys)
    console.log("\n4. Testing Roundtrip (TS Encrypt -> TS Decrypt using Rust Keys)...");
    try {
        const msg = new TextEncoder().encode(fixture.parameters.message_string);
        const tsCiphertext = IBECrypto.ibeEncrypt(mpkBytes, computedIdentity, msg);

        const tsDecrypted = IBECrypto.ibeDecrypt(dkBytes, computedIdentity, mpkBytes, tsCiphertext);
        const tsDecStr = new TextDecoder().decode(tsDecrypted);

        if (tsDecStr === fixture.parameters.message_string) {
            console.log("✅ TS Encryption -> TS Decryption with Rust Keys successful.");
        } else {
            console.log("❌ Roundtrip failed.");
            console.log("Expected:", fixture.parameters.message_string);
            console.log("Got:     ", tsDecStr);
            console.log("Got hex: ", Buffer.from(tsDecrypted).toString('hex'));
            allPassed = false;
        }
    } catch (e) {
        console.log("❌ Roundtrip threw exception:", e);
        allPassed = false;
    }

    if (allPassed) {
        console.log("\n🎉 ALL CHECKS PASSED: TypeScript SDK is compatible with Rust Golden Vectors.");
    } else {
        console.error("\n💥 SOME CHECKS FAILED.");
        process.exit(1);
    }
}

verifyGoldenVectors().catch(console.error);

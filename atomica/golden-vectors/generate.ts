
import { IBECrypto, Ciphertext } from "../timelock-tests/src/ibe-crypto.js";
import { bls12_381 } from "@noble/curves/bls12-381.js";
import * as fs from "fs";
import * as path from "path";

async function generateGoldenVectors() {
    console.log("🔑 Generating IBE Golden Vectors...");

    // 1. Setup Master Keys
    // For golden vectors, we want deterministic output if possible.
    // However, IBECrypto generates random secrets. 
    // We will generate them ONCE here and save them. The "golden" aspect is the saved file.

    const msk = IBECrypto.generateMasterSecret();
    const mpk = IBECrypto.getMasterPublicKey(msk);

    // 2. Define Identity
    const chainId = 4; // Testnet/Devnet
    const interval = 1000n;
    const identity = IBECrypto.computeTimelockIdentity(interval, chainId);

    // 3. Extract Decryption Key (simulating the DKG reveal)
    const dk = IBECrypto.getDecryptionKey(msk, identity);

    // 4. Encrypt a message
    const messageStr = "Golden Vector Message 2024";
    const message = new TextEncoder().encode(messageStr);

    // Note: ibeEncrypt uses random 'r'. We can't control it easily without modifying the library.
    // The ciphertext (u, v) will be different every time this runs.
    // But for a golden vector, we capture ONE valid instance.
    const ciphertext = IBECrypto.ibeEncrypt(mpk, identity, message);

    // 5. Verify Decryption (Self-Check)
    const decrypted = IBECrypto.ibeDecrypt(dk, identity, mpk, ciphertext);
    const decryptedStr = new TextDecoder().decode(decrypted);

    if (decryptedStr !== messageStr) {
        console.error("❌ Integrity Check Failed: Decryption did not match original message.");
        process.exit(1);
    }
    console.log("✅ Integrity Check Passed");

    // 6. Format Output
    const fixture = {
        description: "IBE Golden Vectors for Atomica Timelock",
        timestamp: new Date().toISOString(),
        parameters: {
            chain_id: chainId,
            interval: Number(interval),
            message_string: messageStr,
        },
        keys: {
            msk_hex: Buffer.from(msk).toString('hex'),
            mpk_g2_hex: Buffer.from(mpk).toString('hex'),
            identity_hash_hex: Buffer.from(identity).toString('hex'),
            decryption_key_g1_hex: Buffer.from(dk).toString('hex'),
        },
        ciphertext: {
            u_g2_hex: Buffer.from(ciphertext.u).toString('hex'),
            v_bytes_hex: Buffer.from(ciphertext.v).toString('hex')
        },
        verification: {
            decrypted_hex: Buffer.from(decrypted).toString('hex')
        }
    };

    // 7. Save to JSON
    const outPath = path.join(process.cwd(), "atomica/golden-vectors/ibe_fixtures.json");
    fs.writeFileSync(outPath, JSON.stringify(fixture, null, 2));

    console.log(`💾 Golden Vectors saved to: ${outPath}`);
}

generateGoldenVectors().catch(console.error);

import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters, IBECrypto } from "./index";

async function runIbeFullFlowTest() {
    console.log("🧪 Running IBE Step 3: Full Flow (Encryption & Decryption) Test");

    const testnet = await initializeTestnet(2);
    try {
        const client = new AptosClient(testnet.validatorApiUrl(0));
        const account = testnet.getRootAccount();

        const transactions = new TimelockTransactions(client, account);
        const queries = new TimelockQueries(client);
        const waiters = new TimelockWaiters(queries);

        const intervalSeconds = 5;
        const chainId = 4; // testnet chain id

        // Step 1: Configure shorter interval for testing
        console.log(`Step 1: Setting timelock interval to ${intervalSeconds} seconds`);
        const intervalMicroseconds = intervalSeconds * 1_000_000;
        await transactions.setIntervalForTesting(intervalMicroseconds);
        console.log("✅ Timelock interval configured");

        // Step 2: Wait for rotation to trigger DKG and get MPK
        const initialInterval = await queries.getCurrentInterval();
        const targetInterval = initialInterval + 1;
        console.log(`Step 2: Waiting for rotation to interval ${targetInterval} to trigger DKG`);
        await waiters.waitForIntervalRotation(targetInterval, 300);
        const transcriptBytes = await waiters.waitForPublicKeyPublication(targetInterval, 300);
        if (!transcriptBytes) {
            throw new Error("Transcript not published");
        }
        const mpkG2 = IBECrypto.extractG2FromTranscript(transcriptBytes);
        console.log(`✅ Extracted IBE public key: ${mpkG2.length} bytes`);

        // Step 3: Encrypt a message using IBE
        console.log("Step 3: Encrypting message");
        const message = new TextEncoder().encode("top_secret_bid_1000_atoms");
        const identity = IBECrypto.computeTimelockIdentity(BigInt(targetInterval), chainId); // Use same interval ID for immediate decryption check?
        // Wait, DKG creates MPK for FUTURE use usually, but in this simplified testnet flow, 
        // the MPK might be valid for the current epoch?
        // Actually, identity relies on the interval ID. The decryption key for `targetInterval` 
        // should be revealed when we rotate TO `targetInterval + 1`.

        // Check logic: DKG runs at `targetInterval`. The MPK generated is valid for the system?
        // Actually, in standard timelock, MPK is long-term. Here it's generated per-validator-set-change (epoch)?
        // Assuming MPK is consistent. Let's encrypt for `targetInterval`.

        // We want the decryption key for THIS identity. 
        // In our simplified flow:
        // Interval T: Validators produce MPK (if new epoch) or use existing.
        // Interval T+1: Validators reveal key for T.

        console.log(`✅ Computed identity for interval ${targetInterval}: ${identity.length} bytes`);

        const ciphertext = IBECrypto.ibeEncrypt(mpkG2, identity, message);
        console.log(`✅ Encrypted message: U=${ciphertext.u.length} bytes, V=${ciphertext.v.length} bytes`);

        // Step 4: Wait for reveal (rotation to targetInterval + 1)
        const revealInterval = targetInterval + 1;
        console.log(`Step 4: Waiting for rotation to interval ${revealInterval} to trigger reveal`);
        await waiters.waitForIntervalRotation(revealInterval, 300);
        console.log(`✅ Rotated to reveal interval ${revealInterval}`);

        // Step 5: Fetch revealed Decryption Key
        console.log(`Step 5: Waiting for decryption key reveal for interval ${targetInterval}`);
        const dkBytes = await waiters.waitForSecretAggregation(targetInterval, 2, 60);
        if (!dkBytes) {
            throw new Error("Decryption key not revealed");
        }
        console.log(`✅ Decryption key received: ${dkBytes.length} bytes`);

        // Step 6: Decrypt and verify
        console.log("Step 6: Decrypting and verifying");
        const decrypted = IBECrypto.ibeDecrypt(dkBytes, identity, mpkG2, ciphertext);
        const decryptedText = new TextDecoder().decode(decrypted);
        const originalText = new TextDecoder().decode(message);

        if (decryptedText !== originalText) {
            throw new Error(`Decryption failed: expected "${originalText}", got "${decryptedText}"`);
        }

        console.log("🎉 IBE Step 3 Test PASSED: Message successfully encrypted and decrypted.");
    } finally {
        await performCleanup("IBE Step 3: Full flow test completed");
    }
}

runIbeFullFlowTest().catch(async (e) => {
    console.error(e);
    process.exit(1);
});

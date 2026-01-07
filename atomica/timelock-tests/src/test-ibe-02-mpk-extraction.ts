import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters, IBECrypto } from "./index";

async function runIbeMpkTest() {
    console.log("🧪 Running IBE Step 2: MPK Extraction Test");

    const testnet = await initializeTestnet(2);
    try {
        const client = new AptosClient(testnet.validatorApiUrl(0));
        const account = testnet.getRootAccount();

        const transactions = new TimelockTransactions(client, account);
        const queries = new TimelockQueries(client);
        const waiters = new TimelockWaiters(queries);

        const intervalSeconds = 5;

        // Step 1: Configure shorter interval for testing
        console.log(`Step 1: Setting timelock interval to ${intervalSeconds} seconds`);
        const intervalMicroseconds = intervalSeconds * 1_000_000;
        await transactions.setIntervalForTesting(intervalMicroseconds);
        console.log("✅ Timelock interval configured");

        // Step 2: Wait for rotation to trigger DKG
        const initialInterval = await queries.getCurrentInterval();
        const targetInterval = initialInterval + 1;
        console.log(`Step 2: Waiting for rotation to interval ${targetInterval} to trigger DKG`);
        await waiters.waitForIntervalRotation(targetInterval, 300);
        console.log(`✅ Rotated to interval ${targetInterval}, DKG triggered`);

        // Step 3: Fetch MPK (DKG transcript)
        console.log(`Step 3: Waiting for transcript (MPK) for interval ${targetInterval}`);
        const transcriptBytes = await waiters.waitForPublicKeyPublication(targetInterval, 300);
        if (!transcriptBytes) {
            throw new Error("Transcript not published");
        }
        console.log(`✅ Transcript received: ${transcriptBytes.length} bytes`);

        // Step 4: Extract IBE Public Key (G2 point) from transcript
        console.log("Step 4: Attempting to extract MPK from transcript...");
        try {
            const mpkG2 = IBECrypto.extractG2FromTranscript(transcriptBytes);
            console.log(`✅ Extracted IBE public key: ${mpkG2.length} bytes`);
            if (mpkG2.length !== 96) {
                throw new Error(`Invalid MPK length: expected 96 bytes, got ${mpkG2.length}`);
            }
            // Simple validation it's a valid G2 point (will throw if invalid)
            const point = IBECrypto.deserializeG2(mpkG2);
            console.log("✅ MPK is a valid BLS12-381 G2 point");
            console.log("🎉 IBE Step 2 Test PASSED: MPK extracted successfully.");
        } catch (error) {
            console.error("❌ Failed to extract MPK from transcript:", error);
            throw error;
        }

    } finally {
        await performCleanup("IBE Step 2: MPK Extraction test completed");
    }
}

runIbeMpkTest().catch(async (e) => {
    console.error(e);
    process.exit(1);
});

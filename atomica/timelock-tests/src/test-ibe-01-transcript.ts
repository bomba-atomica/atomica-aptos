import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters } from "./index";

async function runIbeTranscriptTest() {
    console.log("🧪 Running IBE Step 1: Transcript Publication Test");

    const testnet = await initializeTestnet(2);
    try {
        const client = new AptosClient(testnet.validatorApiUrl(0));
        const account = testnet.getRootAccount();

        const transactions = new TimelockTransactions(client, account);
        const queries = new TimelockQueries(client);
        const waiters = new TimelockWaiters(queries);

        const intervalSeconds = 1;

        // Step 1: Configure shorter interval for testing
        console.log(`Step 1: Setting timelock interval to ${intervalSeconds} seconds`);
        const intervalMicroseconds = intervalSeconds * 1_000_000;
        await transactions.setIntervalForTesting(intervalMicroseconds);
        console.log("✅ Timelock interval configured");

        // Step 2: Wait for first rotation to trigger DKG
        const initialInterval = await queries.getCurrentInterval();
        console.log(`Step 2: Waiting for DKG completion (scanning intervals starting from ${initialInterval})`);

        // Scan for 60 seconds
        const scanTimeoutMs = 60000;
        const scanStart = Date.now();
        let transcriptFound = false;

        while (Date.now() - scanStart < scanTimeoutMs) {
            const currentInterval = await queries.getCurrentInterval();
            console.log(`Current interval: ${currentInterval}`);

            // Check recently passed intervals
            for (let i = initialInterval; i <= currentInterval; i++) {
                const transcript = await queries.verifyPublicKeyPublished(i);
                if (transcript) {
                    console.log(`✅ Transcript received for interval ${i}: ${transcript.length} bytes`);
                    transcriptFound = true;
                    break;
                }
            }
            if (transcriptFound) break;
            await new Promise(r => setTimeout(r, 1000));
        }

        if (!transcriptFound) {
            throw new Error(`Transcript not published for any interval since ${initialInterval}`);
        }
        console.log("🎉 IBE Step 1 Test PASSED: Transcript published successfully.");

    } catch (e) {
        console.error("❌ Test failed:", e);
        if (testnet) {
            console.log("Dumping validator logs to file...");
            try {
                const fs = require('fs');
                const path = require('path');
                const logPath = path.resolve(__dirname, '../../validator-logs.txt');
                const p = require("child_process").spawnSync("docker", ["logs", "atomica-validator-0", "--tail", "1000"]);
                fs.writeFileSync(logPath, "=== VALIDATOR 0 LOGS ===\n" + p.stdout.toString() + "\n" + p.stderr.toString());
                console.log(`Logs written to ${logPath}`);
            } catch (err) {
                console.error("Failed to dump logs", err);
            }
        }
        throw e;
    } finally {
        await performCleanup("IBE Step 1: Transcript test completed");
    }
}

runIbeTranscriptTest().catch(async (e) => {
    console.error(e);
    process.exit(1);
});

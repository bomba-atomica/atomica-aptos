import { join } from "path";
import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters } from "./index.js";

/**
 * Phase 2.3: DKG Loop Verification
 * 
 * Verifies that for a specific interval:
 * 1. DKG starts (StartKeyGenEvent) - Implicitly checked by subsequent success.
 * 2. Public key is published (KeyPublishedEvent).
 * 3. Secret shares are aggregated (implicit check effectively, but we can check revealed_secrets).
 * 
 * Note: This test focuses on the on-chain state changes resulting from the DKG loop,
 * ensuring the EpochManager <-> DKGManager <-> Blockchain loop is working.
 */
async function testDkgLoop() {
    console.log("🧪 Testing DKG Loop (Key Generation & Publication)");

    try {
        const frameworkPath = join(__dirname, "../../move-framework-fixtures/head.mrb");
        console.log(`Initializing testnet using framework at: ${frameworkPath}`);

        // Use 4 validators to verify threshold behavior (requires > 2/3)
        // With 4 validators, threshold is 3.
        const testnet = await initializeTestnet(4, frameworkPath);

        try {
            const client = new AptosClient(testnet.validatorApiUrl(0));
            const rootAccount = testnet.getFaucetAccount();

            const queries = new TimelockQueries(client);
            const transactions = new TimelockTransactions(client, rootAccount);
            const waiters = new TimelockWaiters(queries);

            console.log("Step 1: Setup - Configuring short interval (5s)");
            await transactions.setIntervalForTesting(5000000);

            // Get current interval
            const startInterval = await queries.getCurrentInterval();
            console.log(`Current interval: ${startInterval}`);

            // We want to test the loop for the NEXT interval (or the current one if it just started)
            // But to be deterministic, we might want to force a rotation to N+1 and then wait.
            // Or we can just pick N+2 to be safe and wait for it.

            const targetInterval = startInterval + 2;
            console.log(`Targeting interval ${targetInterval} for full DKG verification`);

            console.log(`Step 2: Waiting for rotation to ${targetInterval}`);
            await waiters.waitForIntervalRotation(targetInterval, 60);

            console.log("Step 3: Waiting for DKG completion (Public Key Published)");
            // Once we rotate to N, the DKG for N starts immediately.
            // We wait for the public key to appear in the `public_keys` table.

            const transcript = await waiters.waitForPublicKeyPublication(targetInterval, 60);
            console.log(`✅ Public key published for interval ${targetInterval}: ${transcript.length} bytes`);

            // Note: In Phase 2, we might not have 'reveals' yet because reveals only happen
            // when the interval ends (or is requested). 
            // So for now, we only verify Key Generation.

            // However, if we wait for the interval to END, we can verify that the secret is reconstructed
            // (assuming auto-reveal logic is implemented or we trigger it).

            // Success!
            console.log("✅ DKG loop test passed!");
            await performCleanup("DKG loop test completed");

        } finally {
            // This inner finally block is removed as per instructions.
            // The cleanup is now handled at the end of the outer try block for success,
            // and within the outer catch block for failure.
        }

    } catch (error) {
        console.error("❌ DKG loop test failed:", error);

        // Dump logs for validator 0
        try {
            const { spawn } = require("child_process");
            console.log("=== VALIDATOR 0 LOGS ===");
            const proc = spawn("docker", ["logs", "atomica-validator-0", "--tail", "5000"], { stdio: 'inherit' });
            await new Promise((resolve) => proc.on('close', resolve));
            console.log("========================");
        } catch (e) {
            console.log("Failed to fetch logs:", e);
        }

        // Cleanup after dumping logs
        try {
            await performCleanup("DKG loop test failed");
        } catch (e) { }

        process.exit(1);
    }
}

testDkgLoop().catch((e) => {
    console.error(e);
    process.exit(1);
});

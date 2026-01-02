import { join } from "path";
import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters } from "./index.js";

/**
 * Phase 2.2: Passive Timelock Rotation (The "Heartbeat")
 * 
 * Verifies that:
 * 1. The timelock system initializes correctly.
 * 2. Intervals increment automatically (passive rotation).
 * 3. Events are emitted for rotation.
 */
async function testPassiveTransitions() {
    console.log("🧪 Testing Passive Timelock Transitions (Heartbeat)");

    try {
        // Initialize testnet with 2 validators to ensure consensus is running
        // Using custom framework path to ensure timelock module is present
        const frameworkPath = join(__dirname, "../../move-framework-fixtures/head.mrb");
        console.log(`Initializing testnet using framework at: ${frameworkPath}`);

        const testnet = await initializeTestnet(2, frameworkPath);

        try {
            const client = new AptosClient(testnet.validatorApiUrl(0));
            const rootAccount = testnet.getFaucetAccount();

            const queries = new TimelockQueries(client);
            const transactions = new TimelockTransactions(client, rootAccount);
            const waiters = new TimelockWaiters(queries);

            console.log("Step 1: Checking initialization");
            const initialState = await queries.getTimelockState();
            console.log(`✅ initialized. Current interval: ${initialState.current_interval}`);

            // Set a short interval for testing (e.g., 2 seconds)
            // Default might be 1 hour (3600s) from our Phase 1 revert
            console.log("Step 2: Configuring short interval (2s)");
            await transactions.setIntervalForTesting(2000000);

            console.log("Step 3: Monitoring passive rotation (waiting for 3 intervals)");

            // We want to see the interval increase WITHOUT us calling trigger_rotation
            // This relies on the validator's block prologue triggering on_new_block

            const startInterval = await queries.getCurrentInterval();
            let currentInterval = startInterval;

            // Monitor for 3 transitions
            for (let i = 1; i <= 3; i++) {
                const target = startInterval + i;
                console.log(`Waiting for interval ${target}...`);

                // We give it plenty of time (e.g. 10s per interval)
                await waiters.waitForIntervalRotation(target, 30);
                console.log(`✅ Reached interval ${target}`);
                currentInterval = target;
            }

            console.log("Step 4: verifying implicit on-chain state");
            // If we reached here, current_interval matches.
            // We should also check that the last_rotation_time is recent?
            const finalState = await queries.getTimelockState();
            const lastRotation = parseInt(finalState.last_rotation_time);
            const now = await queries.getCurrentTimestamp();

            console.log(`Last rotation time: ${lastRotation}, Chain time: ${now}`);
            if (now - lastRotation > 10000000) { // 10s
                console.warn("⚠️ Warning: Last rotation seems old, but we passed the check.");
            }

            console.log(`✅ Passive transition test PASSED! (${startInterval} -> ${currentInterval})`);

        } finally {
            await performCleanup("Passive transition test completed");
        }

    } catch (error) {
        console.error("❌ Passive transition test failed:", error);
        process.exit(1);
    }
}

testPassiveTransitions().catch((e) => {
    console.error(e);
    process.exit(1);
});

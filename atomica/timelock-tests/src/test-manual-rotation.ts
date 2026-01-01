import { join } from "path";
import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters } from "./index.js";

/**
 * Test manual rotation trigger functionality
 */
async function testManualRotation() {
  console.log("🧪 Testing Manual Rotation Trigger");

  try {
    // Initialize testnet
    console.log("Initializing testnet with 2 validators...");
    // We explicitly pass the path to our custom-built framework (head.mrb).
    // These fixtures are built with 'aptos-framework custom' and include modified configs
    // (like 0.1s interval) essential for this test.
    // DockerTestnet will mount this file to /framework.mrb, enabling custom genesis.
    const frameworkPath = join(__dirname, "../../move-framework-fixtures/head.mrb");
    const testnet = await initializeTestnet(2, frameworkPath);

    try {
      const client = new AptosClient(testnet.validatorApiUrl(0));
      const rootAccount = testnet.getFaucetAccount();

      const queries = new TimelockQueries(client);
      const transactions = new TimelockTransactions(client, rootAccount);
      const waiters = new TimelockWaiters(queries);

      console.log("Step 1: Checking timelock initialization");
      const timelockState = await queries.getTimelockState();
      console.log(`✅ Timelock initialized - current interval: ${timelockState.current_interval}`);

      console.log("Step 2: Setting short interval for testing");
      await transactions.setIntervalForTesting(100000); // 0.1 second

      const configuredInterval = await queries.getConfiguredInterval();
      console.log(`🔍 DEBUG: Configured interval is: ${configuredInterval}`);

      console.log("✅ Timelock interval set to 0.1 second");

      console.log("Step 3: Manually triggering rotation");
      // Wait for interval to pass (0.1s configured, wait 2s to be safe)
      console.log("Waiting 2s for interval to pass...");
      await new Promise((resolve) => setTimeout(resolve, 2000));

      await transactions.triggerRotation();
      console.log("✅ Manual rotation triggered");

      console.log("Step 4: Waiting for rotation to complete");
      await waiters.waitForIntervalRotation(1, 30);
      console.log("✅ Rotation completed - advanced to interval 1");

      console.log("Step 5: Verifying new interval");
      const newState = await queries.getTimelockState();
      if (newState.current_interval === 1) {
        console.log("✅ Manual rotation test PASSED!");
      } else {
        console.log(`❌ Manual rotation test FAILED - expected interval 1, got ${newState.current_interval}`);
      }
    } finally {
      await performCleanup("Manual rotation test completed");
    }
  } catch (error) {
    console.error("❌ Manual rotation test failed:", error);
    process.exit(1);
  }
}

testManualRotation().catch((e) => {
  console.error(e);
  process.exit(1);
});

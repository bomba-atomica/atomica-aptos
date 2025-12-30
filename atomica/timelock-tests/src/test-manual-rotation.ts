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
    const testnet = await initializeTestnet(2);

    try {
      const client = new AptosClient(testnet.validatorApiUrl(0));
      const rootAccount = testnet.getFaucetAccount();

      const queries = new TimelockQueries(client);
      const transactions = new TimelockTransactions(client, rootAccount);
      const waiters = new TimelockWaiters(client);

      console.log("Step 1: Checking timelock initialization");
      const timelockState = await queries.getTimelockState(client);
      console.log(`✅ Timelock initialized - current interval: ${timelockState.current_interval}`);

      console.log("Step 2: Setting short interval for testing");
      await transactions.setIntervalForTesting(1000000); // 1 second
      console.log("✅ Timelock interval set to 1 second");

      console.log("Step 3: Manually triggering rotation");
      await transactions.triggerRotation();
      console.log("✅ Manual rotation triggered");

      console.log("Step 4: Waiting for rotation to complete");
      await waiters.waitForInterval(client, 1, 30);
      console.log("✅ Rotation completed - advanced to interval 1");

      console.log("Step 5: Verifying new interval");
      const newState = await queries.getTimelockState(client);
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
    throw error;
  }
}

testManualRotation().catch(console.error);

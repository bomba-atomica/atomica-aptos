import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient, AptosAccount } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters } from "./index";

async function runBasicFlowTest() {
  console.log("🧪 Running Basic Timelock Flow Test");

  const testnet = await initializeTestnet(4);
  try {
    const client = new AptosClient(testnet.validatorApiUrl(0));
    const account = testnet.getRootAccount();

    const transactions = new TimelockTransactions(client, account);
    const queries = new TimelockQueries(client);
    const waiters = new TimelockWaiters(queries);

    const intervalSeconds = 5;

    // Step 1: Verify timelock initialized at genesis
    console.log("Step 1: Checking timelock initialization");
    const initialized = await queries.isTimelockInitialized();
    if (!initialized) {
      throw new Error("Timelock should be initialized at genesis");
    }
    console.log("✅ Timelock initialized at genesis");

    const initialInterval = await queries.getCurrentInterval();
    console.log(`Initial interval: ${initialInterval}`);

    // Step 2: Configure shorter interval for testing
    console.log(`Step 2: Setting timelock interval to ${intervalSeconds} seconds`);
    const intervalMicroseconds = intervalSeconds * 1_000_000;
    await transactions.setIntervalForTesting(intervalMicroseconds);
    console.log("✅ Timelock interval configured");

    // Step 3: Wait for first interval rotation
    console.log("Step 3: Waiting for first interval rotation");
    const targetInterval = initialInterval + 1;
    const rotationResult = await waiters.waitForIntervalRotation(targetInterval, 120);
    if (rotationResult.current_interval < targetInterval) {
      throw new Error(`Expected interval ${targetInterval}, got ${rotationResult.current_interval}`);
    }
    console.log(`✅ Rotated to interval ${rotationResult.current_interval}`);

    // Step 4: Verify public key published
    console.log(`Step 4: Waiting for public key publication for interval ${targetInterval}`);
    const transcript = await waiters.waitForPublicKeyPublication(targetInterval, 60);
    if (!transcript || transcript.length === 0) {
      throw new Error("Public key not published");
    }
    console.log(`✅ Public key published: ${transcript.length} bytes`);

    // Step 5: Wait for reveal rotation
    const revealInterval = targetInterval + 1;
    console.log(`Step 5: Waiting for reveal rotation to interval ${revealInterval}`);
    await waiters.waitForIntervalRotation(revealInterval, 120);
    console.log(`✅ Rotated to reveal interval ${revealInterval}`);

    // Step 6: Verify secret aggregation
    console.log(`Step 6: Waiting for secret aggregation for interval ${targetInterval}`);
    const secret = await waiters.waitForSecretAggregation(targetInterval, 3, 60); // threshold = 3 for 4 validators
    if (!secret || secret.length === 0) {
      throw new Error("Secret not aggregated");
    }
    console.log(`✅ Secret aggregated: ${secret.length} bytes`);

    console.log("🎉 Basic timelock flow test PASSED!");
  } finally {
    await performCleanup("Basic timelock flow test completed");
  }
}

runBasicFlowTest().catch(console.error);

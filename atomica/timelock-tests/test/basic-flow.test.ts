import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle.js";
import { AptosClient, AptosAccount } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters } from "../src";

describe("Basic Timelock Flow", () => {
  let testnet: any;
  const NUM_VALIDATORS = 4;

  beforeAll(async () => {
    testnet = await initializeTestnet(NUM_VALIDATORS);
  }, 300000); // 5 min timeout

  afterAll(async () => {
    await performCleanup("Basic timelock flow test completed");
  });

  /**
   * Test basic timelock flow with fast interval for testing.
   *
   * This test verifies the end-to-end flow of timelock encryption:
   * 1. Genesis initialization of timelock system
   * 2. Interval rotation triggers DKG for new keys
   * 3. Validators publish public key for encryption
   * 4. Interval rotation triggers reveal request
   * 5. Validators reveal secret shares
   * 6. On-chain aggregation produces decryption key
   *
   * Implementation details:
   * - Starts a 4-validator network using docker-test-harness
   * - Verifies timelock is initialized at genesis by checking blockchain state
   * - Configures shorter interval for testing via transaction
   * - Waits for first rotation and verifies public key publication
   * - Waits for reveal and checks secret aggregation
   */
  it("should execute complete timelock flow", async () => {
    const client = new AptosClient(testnet.validatorApiUrl(0));
    const account = testnet.getRootAccount();

    const transactions = new TimelockTransactions(client, account);
    const queries = new TimelockQueries(client);
    const waiters = new TimelockWaiters(queries);

    const intervalSeconds = 5;

    // Step 1: Verify timelock initialized at genesis
    const initialized = await queries.isTimelockInitialized();
    expect(initialized).toBe(true);

    const initialInterval = await queries.getCurrentInterval();
    console.log(`Initial interval: ${initialInterval}`);

    // Step 2: Configure shorter interval for testing
    console.log(`Setting timelock interval to ${intervalSeconds} seconds`);
    const intervalMicroseconds = intervalSeconds * 1_000_000;
    await transactions.setIntervalForTesting(intervalMicroseconds);
    console.log("Timelock interval configured successfully");

    // Step 3: Wait for first interval rotation
    console.log("Waiting for first interval rotation");
    const targetInterval = initialInterval + 1;
    const rotationResult = await waiters.waitForIntervalRotation(targetInterval, 120);
    expect(rotationResult.current_interval).toBeGreaterThanOrEqual(targetInterval);

    // Step 4: Verify public key published
    console.log(`Waiting for public key publication for interval ${targetInterval}`);
    const transcript = await waiters.waitForPublicKeyPublication(targetInterval, 60);
    expect(transcript).toBeTruthy();
    expect(transcript!.length).toBeGreaterThan(0);
    console.log(`Public key published for interval ${targetInterval}: ${transcript!.length} bytes`);

    // Step 5: Wait for reveal rotation
    const revealInterval = targetInterval + 1;
    console.log(`Waiting for reveal rotation to interval ${revealInterval}`);
    await waiters.waitForIntervalRotation(revealInterval, 120);

    // Step 6: Verify secret aggregation
    console.log(`Waiting for secret aggregation for interval ${targetInterval}`);
    const secret = await waiters.waitForSecretAggregation(targetInterval, 3, 60); // threshold = 3 for 4 validators
    expect(secret).toBeTruthy();
    expect(secret!.length).toBeGreaterThan(0);
    console.log(`Secret aggregated for interval ${targetInterval}: ${secret!.length} bytes`);

    console.log("✅ Basic timelock flow test passed");
  });
});

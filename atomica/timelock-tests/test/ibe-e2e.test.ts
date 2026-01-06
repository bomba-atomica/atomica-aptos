import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle.js";
import { AptosClient, AptosAccount } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters, IBECrypto } from "../src";

describe("IBE Encrypt/Decrypt E2E", () => {
  let testnet: any;
  const NUM_VALIDATORS = 4;

  beforeAll(async () => {
    testnet = await initializeTestnet(NUM_VALIDATORS);
  }, 300000); // 5 min timeout

  afterAll(async () => {
    await performCleanup("IBE E2E test completed");
  });

  /**
   * IBE Encrypt/Decrypt E2E test
   *
   * This test verifies the complete Identity-Based Encryption (IBE) flow:
   * 1. Fetch the published MPK (DKG transcript) from the chain
   * 2. Extract the actual IBE public key from the transcript
   * 3. Encrypt a message using IBE with timelock identity
   * 4. Wait for the DK (decryption key) to be revealed
   * 5. Successfully decrypt the message and verify integrity
   *
   * Implementation details:
   * - Setup 4-validator testnet with docker-test-harness
   * - Configure short timelock interval for testing
   * - Wait for DKG to complete and transcript publication
   * - Deserialize transcript to extract IBE public key (G2 point)
   * - Encrypt test message using IBE
   * - Wait for interval rotation to trigger secret reveal
   * - Fetch revealed decryption key (G1 point)
   * - Decrypt message and verify it matches original
   */
  it("should encrypt and decrypt message using IBE", async () => {
    const client = new AptosClient(testnet.validatorApiUrl(0));
    const account = testnet.getRootAccount();

    const transactions = new TimelockTransactions(client, account);
    const queries = new TimelockQueries(client);
    const waiters = new TimelockWaiters(queries);

    const intervalSeconds = 5;

    // Step 1: Configure shorter interval for testing
    console.log(`Setting timelock interval to ${intervalSeconds} seconds`);
    const intervalMicroseconds = intervalSeconds * 1_000_000;
    await transactions.setIntervalForTesting(intervalMicroseconds);
    console.log("Timelock interval configured successfully");

    // Step 2: Wait for first rotation to trigger DKG
    const initialInterval = await queries.getCurrentInterval();
    const targetInterval = initialInterval + 1;
    console.log(`Waiting for rotation to interval ${targetInterval} to trigger DKG`);
    await waiters.waitForIntervalRotation(targetInterval, 120);

    // Step 3: Fetch MPK (DKG transcript)
    console.log(`Waiting for transcript (MPK) for interval ${targetInterval}`);
    const transcriptBytes = await waiters.waitForPublicKeyPublication(targetInterval, 60);
    expect(transcriptBytes).toBeTruthy();

    // Step 4: Extract IBE Public Key (G2 point) from transcript
    const mpkG2 = IBECrypto.extractG2FromTranscript(transcriptBytes);

    // Step 5: Encrypt a message using IBE
    const message = new TextEncoder().encode("top_secret_bid_1000_atoms");
    const timelockId = BigInt(targetInterval);
    const deadlineTimestampMicroseconds = BigInt(Date.now() * 1000 + 3600_000_000); // 1 hour from now
    const identity = IBECrypto.computeTimelockIdentity(timelockId, deadlineTimestampMicroseconds);

    const ciphertext = IBECrypto.ibeEncrypt(mpkG2, identity, message);

    console.log(`Message encrypted for timelock ${timelockId}`);

    // Step 6: Wait for reveal (rotation to targetInterval + 1)
    const revealInterval = targetInterval + 1;
    console.log(`Waiting for rotation to interval ${revealInterval} to trigger reveal`);
    await waiters.waitForIntervalRotation(revealInterval, 120);

    // Step 7: Fetch revealed Decryption Key (G1 point)
    console.log(`Waiting for decryption key reveal for interval ${targetInterval}`);
    const dkBytes = await waiters.waitForSecretAggregation(targetInterval, 3, 60);
    expect(dkBytes).toBeTruthy();

    const dkG1 = dkBytes; // ibeDecrypt expects bytes and handles deserialization

    // Step 8: Decrypt and verify
    const decrypted = IBECrypto.ibeDecrypt(dkG1, identity, mpkG2, ciphertext);
    expect(new TextDecoder().decode(decrypted)).toBe(new TextDecoder().decode(message));

    console.log("✅ IBE E2E test passed! Message successfully encrypted and decrypted.");
  });
});

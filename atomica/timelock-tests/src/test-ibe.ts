import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient, AptosAccount } from "aptos";
import { TimelockTransactions, TimelockQueries, TimelockWaiters, IBECrypto } from "./index";

async function runIbeE2eTest() {
  console.log("🧪 Running IBE Encrypt/Decrypt E2E Test");

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

    // Step 2: Wait for first rotation to trigger DKG
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
    const mpkG2 = IBECrypto.extractG2FromTranscript(transcriptBytes);
    console.log(`✅ Extracted IBE public key: ${mpkG2.length} bytes`);

    // Step 5: Encrypt a message using IBE
    const message = new TextEncoder().encode("top_secret_bid_1000_atoms");
    const identity = IBECrypto.computeTimelockIdentity(BigInt(targetInterval), chainId);
    console.log(`✅ Computed identity for interval ${targetInterval}: ${identity.length} bytes`);

    const ciphertext = IBECrypto.ibeEncrypt(mpkG2, identity, message);
    console.log(`✅ Encrypted message: U=${ciphertext.u.length} bytes, V=${ciphertext.v.length} bytes`);
    console.log(`✅ Message encrypted for interval ${targetInterval}`);

    // Step 6: Wait for reveal (rotation to targetInterval + 1)
    const revealInterval = targetInterval + 1;
    console.log(`Step 6: Waiting for rotation to interval ${revealInterval} to trigger reveal`);
    await waiters.waitForIntervalRotation(revealInterval, 300);
    console.log(`✅ Rotated to reveal interval ${revealInterval}`);

    // Step 7: Fetch revealed Decryption Key (G1 point)
    console.log(`Step 7: Waiting for decryption key reveal for interval ${targetInterval}`);
    const dkBytes = await waiters.waitForSecretAggregation(targetInterval, 2, 60);
    if (!dkBytes) {
      throw new Error("Decryption key not revealed");
    }
    console.log(`✅ Decryption key received: ${dkBytes.length} bytes`);

    // Step 8: Decrypt and verify
    // Pass raw dkBytes as ibeDecrypt expects Uint8Array (and parses it internally)
    const decrypted = IBECrypto.ibeDecrypt(dkBytes, identity, mpkG2, ciphertext);
    const decryptedText = new TextDecoder().decode(decrypted);
    const originalText = new TextDecoder().decode(message);

    if (decryptedText !== originalText) {
      throw new Error(`Decryption failed: expected "${originalText}", got "${decryptedText}"`);
    }

    console.log("🎉 IBE E2E test PASSED! Message successfully encrypted and decrypted.");
  } finally {
    // await performCleanup("IBE E2E test completed");
  }
}

runIbeE2eTest().catch(async (e) => {
  console.error(e);
  console.log("Debug mode: Keeping testnet alive for inspection. Check logs with 'docker logs validator-0'");
  await new Promise(r => setTimeout(r, 600000)); // Wait 10 mins
  process.exit(1);
});

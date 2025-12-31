import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient, AptosAccount } from "aptos";

async function testTransaction() {
  console.log("🧪 Testing transaction submission");

  const testnet = await initializeTestnet(2);
  try {
    const client = new AptosClient(testnet.validatorApiUrl(0));
    const account = testnet.getRootAccount();

    console.log(`Account: ${account.address().hex()}`);

    // Try a simple transfer like probe_faucet.ts
    const newAccount = new AptosAccount();
    console.log(`New account: ${newAccount.address().hex()}`);

    const payload = {
      type: "entry_function_payload",
      function: "0x1::aptos_account::transfer",
      type_arguments: [],
      arguments: [newAccount.address().hex(), "1000000"], // 0.01 APT
    };

    console.log("Generating transaction...");
    const txn = await client.generateTransaction(account.address(), payload);
    console.log("Transaction generated");

    console.log("Signing transaction...");
    const signedTxn = await client.signTransaction(account, txn);
    console.log("Transaction signed");

    console.log("Submitting transaction...");
    const pending = await client.submitTransaction(signedTxn);
    console.log("Transaction submitted:", pending.hash);

    console.log("Waiting for transaction...");
    const txnResult = await client.waitForTransactionWithResult(pending.hash);
    console.log("✓ Transaction completed!");
    console.log("Transaction hash:", pending.hash);
  } catch (error) {
    console.error("❌ Transaction failed:", error);
  } finally {
    await performCleanup("Transaction test completed");
  }
}

testTransaction();

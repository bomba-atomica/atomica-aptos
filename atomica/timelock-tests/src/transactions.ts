import { AptosClient, AptosAccount, Types } from "aptos";

/**
 * Helper functions for crafting timelock-related transactions
 */

export class TimelockTransactions {
  constructor(
    private client: AptosClient,
    private account: AptosAccount,
  ) {}

  /**
   * Set timelock interval for testing (only works on testnet)
   */
  async setIntervalForTesting(intervalMicroseconds: number): Promise<string> {
    console.log(`Setting interval to ${intervalMicroseconds} microseconds`);

    const payload = {
      type: "entry_function_payload",
      function: "0x1::timelock_config::set_interval_for_testing",
      type_arguments: [],
      arguments: [intervalMicroseconds.toString()],
    };

    console.log(`Generating transaction for account ${this.account.address()}`);
    const txn = await this.client.generateTransaction(this.account.address(), payload);
    console.log("Transaction generated");

    console.log("Signing transaction...");
    const signedTxn = await this.client.signTransaction(this.account, txn);
    console.log("Transaction signed");

    console.log("Submitting transaction...");
    const pendingTxn = await this.client.submitTransaction(signedTxn);
    console.log("Transaction submitted:", pendingTxn.hash);

    console.log("Waiting for transaction...");
    const txnResult = await this.client.waitForTransactionWithResult(pendingTxn.hash);
    console.log("✓ Transaction completed!");
    console.log("Result type:", typeof txnResult, "keys:", Object.keys(txnResult || {}));

    // The waitForTransactionWithResult throws on failure, so if we get here it's successful

    return pendingTxn.hash;
  }

  /**
   * Manually trigger timelock rotation (can be called by anyone after scheduled time)
   */
  async triggerRotation(): Promise<string> {
    const payload: Types.TransactionPayload = {
      type: "entry_function_payload",
      function: "0x1::timelock::trigger_rotation",
      type_arguments: [],
      arguments: [],
    };

    const txn = await this.client.generateTransaction(this.account.address(), payload);
    const signedTxn = await this.client.signTransaction(this.account, txn);
    const pendingTxn = await this.client.submitTransaction(signedTxn);
    const txnResult = await this.client.waitForTransactionWithResult(pendingTxn.hash);

    if (txnResult.success === false || txnResult.vm_status !== "Executed successfully") {
      throw new Error(`Rotation trigger failed: ${txnResult.vm_status || "Unknown error"}`);
    }

    return pendingTxn.hash;
  }
}

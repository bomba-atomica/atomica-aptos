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

    const payload: Types.TransactionPayload = {
      type: "entry_function_payload",
      function: "0x1::timelock_config::set_interval_for_testing",
      type_arguments: [],
      arguments: [intervalMicroseconds.toString()],
    };

    console.log(`Generating transaction for account ${this.account.address()}`);
    const txnRequest = await this.client.generateTransaction(this.account.address(), payload);
    console.log("Transaction generated, signing...");
    const signedTxn = await this.client.signTransaction(this.account, txnRequest);
    console.log("Transaction signed, submitting...");
    const txnResponse = await this.client.submitTransaction(signedTxn);
    console.log(`Transaction submitted: ${txnResponse.hash}, waiting for confirmation...`);
    await this.client.waitForTransaction(txnResponse.hash);
    console.log("Transaction confirmed successfully");

    return txnResponse.hash;
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

    console.log("Rotation trigger transaction completed");

    return pendingTxn.hash;
  }
}

import { AptosClient, AptosAccount, Types } from "aptos";

/**
 * Helper functions for crafting timelock-related transactions
 */

export class TimelockTransactions {
  constructor(
    private client: AptosClient,
    private account: AptosAccount,
  ) { }

  /**
   * Set the timelock interval for testing purposes.
   * Note: This is only available on non-mainnet chains.
   */
  async setIntervalForTesting(intervalUs: number): Promise<string> {
    const payload: Types.TransactionPayload = {
      type: "entry_function_payload",
      function: "0x1::timelock_config::set_interval_for_testing",
      type_arguments: [],
      arguments: [intervalUs],
    };

    const txn = await this.client.generateTransaction(this.account.address(), payload);
    const signedTxn = await this.client.signTransaction(this.account, txn);
    const pendingTxn = await this.client.submitTransaction(signedTxn);
    const txnResult = (await this.client.waitForTransactionWithResult(pendingTxn.hash)) as any;

    if (!txnResult.success) {
      throw new Error(`Set interval for testing failed: ${txnResult.vm_status}`);
    }

    console.log(`Set interval for testing transaction completed: ${intervalUs}us`);

    return pendingTxn.hash;
  }


  /**
   * Force force rotation for testing purposes.
   * Note: This is only available on non-mainnet chains.
   */
  async forceRotationForTesting(): Promise<string> {
    const payload: Types.TransactionPayload = {
      type: "entry_function_payload",
      function: "0x1::timelock::force_rotation_for_testing",
      type_arguments: [],
      arguments: [],
    };

    const txn = await this.client.generateTransaction(this.account.address(), payload);
    const signedTxn = await this.client.signTransaction(this.account, txn);
    const pendingTxn = await this.client.submitTransaction(signedTxn);
    const txnResult = (await this.client.waitForTransactionWithResult(pendingTxn.hash)) as any;

    if (!txnResult.success) {
      throw new Error(`Force rotation for testing failed: ${txnResult.vm_status}`);
    }

    console.log(`Force rotation for testing transaction completed`);

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
    const txnResult = await this.client.waitForTransactionWithResult(pendingTxn.hash) as any;

    if (!txnResult.success) {
      throw new Error(`Rotation trigger failed: ${txnResult.vm_status}`);
    }

    console.log("Rotation trigger transaction completed");

    return pendingTxn.hash;
  }
}

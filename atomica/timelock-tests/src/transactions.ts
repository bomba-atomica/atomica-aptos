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

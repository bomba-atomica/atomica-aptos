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
    const payload: Types.TransactionPayload = {
      type: "entry_function_payload",
      function: "0x1::timelock_config::set_interval_for_testing",
      type_arguments: [],
      arguments: [intervalMicroseconds.toString()],
    };

    const txnRequest = await this.client.generateTransaction(this.account.address(), payload);
    const signedTxn = await this.client.signTransaction(this.account, txnRequest);
    const txnResponse = await this.client.submitTransaction(signedTxn);
    await this.client.waitForTransaction(txnResponse.hash);

    return txnResponse.hash;
  }
}

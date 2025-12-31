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

    try {
      console.log(`Generating transaction for account ${this.account.address()}`);
      const txnRequest = await this.client.generateTransaction(this.account.address(), payload);
      console.log(`Transaction generated, signing...`);
      const signedTxn = await this.client.signTransaction(this.account, txnRequest);
      console.log(`Transaction signed, submitting...`);
      const txnResponse = await this.client.submitTransaction(signedTxn);
      console.log(`Transaction submitted: ${txnResponse.hash}, waiting for confirmation...`);
      await this.client.waitForTransaction(txnResponse.hash);
      console.log(`Transaction confirmed successfully`);
      return txnResponse.hash;
    } catch (error) {
      console.error(`Transaction failed: ${error}`);
      throw error;
    }

    return txnResponse.hash;
  }
}

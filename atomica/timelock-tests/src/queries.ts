import { AptosClient } from "aptos";

/**
 * Helper functions for querying timelock blockchain state
 */

export class TimelockQueries {
  constructor(private client: AptosClient) {}

  /**
   * Get current timelock interval
   */
  async getCurrentInterval(): Promise<number> {
    const resources = await this.client.getAccountResources("0x1");
    const timelockResource = resources.find((r: any) => r.type.includes("timelock::TimelockState"));
    if (!timelockResource) {
      throw new Error("TimelockState resource not found");
    }
    return parseInt((timelockResource.data as any).current_interval, 10);
  }

  /**
   * Get timelock state for debugging
   */
  async getTimelockState(): Promise<any> {
    const resources = await this.client.getAccountResources("0x1");
    const timelockResource = resources.find((r: any) => r.type.includes("timelock::TimelockState"));
    if (!timelockResource) {
      throw new Error("TimelockState resource not found");
    }
    return timelockResource.data;
  }

  /**
   * Check if timelock is initialized
   */
  async isTimelockInitialized(): Promise<boolean> {
    try {
      await this.getCurrentInterval();
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Verify public key published for interval
   */
  async verifyPublicKeyPublished(interval: number): Promise<Uint8Array | null> {
    try {
      const resources = await this.client.getAccountResources("0x1");
      const dsaResource = resources.find((r: any) => r.type.includes("threshold_dsa::State"));

      if (!dsaResource) {
        // Fallback for transition period or if test setup hasn't init dsa yet?
        // But initialize() does it.
        return null;
      }

      const handle = (dsaResource.data as any).master_public_keys.handle;

      const item = await this.client.getTableItem(handle, {
        key_type: "u64",
        value_type: "vector<u8>",
        key: interval.toString(),
      });

      // item is the value directly. value_type is vector<u8>, returned as hex string in updated SDKs?
      // Aptos TS SDK returns hex string for vector<u8> usually.
      if (item) {
        if (typeof item === "string") {
          // Hex string
          return new Uint8Array(Buffer.from(item.startsWith("0x") ? item.slice(2) : item, "hex"));
        } else if (Array.isArray(item)) {
          // Array of numbers
          return new Uint8Array(item);
        }
        return item as Uint8Array; // Just in case
      }
    } catch (error: any) {
      if (error?.status === 404 || error?.message?.includes("Table item not found")) {
        return null;
      }
      // console.log("Error fetching public key table item:", error);
    }
    return null;
  }

  /**
   * Verify secret aggregated for interval
   * Checks revealed_secrets table.
   * Note: threshold arg is ignored because if it's in revealed_secrets, it met the threshold.
   */
  async verifySecretAggregated(interval: number, threshold: number): Promise<Uint8Array | null> {
    try {
      const state = await this.getTimelockState();
      const handle = (state as any).decryption_keys.handle;

      const item = await this.client.getTableItem(handle, {
        key_type: "u64",
        value_type: "vector<u8>",
        key: interval.toString(),
      });

      if (item) {
        if (typeof item === "string") {
          return new Uint8Array(Buffer.from(item.startsWith("0x") ? item.slice(2) : item, "hex"));
        } else if (Array.isArray(item)) {
          return new Uint8Array(item);
        }
        return item as Uint8Array;
      }
    } catch (error: any) {
      if (error?.status === 404 || error?.message?.includes("Table item not found")) {
        return null;
      }
    }
    return null;
  }

  /**
   * Get current blockchain timestamp
   */
  async getCurrentTimestamp(): Promise<number> {
    const resources = await this.client.getAccountResources("0x1");
    const timestampResource = resources.find((r: any) => r.type.includes("timestamp::CurrentTimeMicroseconds"));
    if (!timestampResource) {
      throw new Error("Timestamp resource not found");
    }
    return parseInt((timestampResource.data as any).microseconds, 10);
  }

  /**
   * Get configured timelock checkpoint period
   */
  async getConfiguredInterval(): Promise<number> {
    try {
      const resources = await this.client.getAccountResources("0x1");
      const configResource = resources.find((r: any) => r.type.includes("timelock_config::TimelockConfig"));
      if (!configResource) {
        throw new Error("Timelock config resource not found");
      }
      return parseInt((configResource.data as any).checkpoint_period_microseconds, 10);
    } catch (error) {
      console.log(`Error getting configured checkpoint period: ${error}`);
      throw error;
    }
  }
}

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
    return (timelockResource.data as any).current_interval;
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
      const transcriptResource = resources.find(
        (r: any) => r.type.includes("timelock::TranscriptStore") && (r.data as any).interval === interval,
      );
      if (transcriptResource) {
        return new Uint8Array((transcriptResource.data as any).transcript);
      }
    } catch (error) {
      // Resource not found or other error
    }
    return null;
  }

  /**
   * Verify secret aggregated for interval
   */
  async verifySecretAggregated(interval: number, threshold: number): Promise<Uint8Array | null> {
    try {
      const resources = await this.client.getAccountResources("0x1");
      const secretResource = resources.find(
        (r: any) => r.type.includes("timelock::DecryptionKeyStore") && (r.data as any).interval === interval,
      );
      if (secretResource && (secretResource.data as any).share_count >= threshold) {
        return new Uint8Array((secretResource.data as any).decryption_key);
      }
    } catch (error) {
      // Resource not found or other error
    }
    return null;
  }
}

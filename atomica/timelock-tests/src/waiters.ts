import { TimelockQueries } from "./queries";

/**
 * Helper functions for waiting on timelock events
 */

export class TimelockWaiters {
  constructor(private queries: TimelockQueries) {}

  /**
   * Wait for interval rotation
   */
  async waitForIntervalRotation(
    targetInterval: number,
    timeoutSeconds: number = 120,
  ): Promise<{ current_interval: number }> {
    const startTime = Date.now();
    const timeoutMs = timeoutSeconds * 1000;

    console.log(`Waiting for interval to reach ${targetInterval} (timeout: ${timeoutSeconds}s)`);

    while (Date.now() - startTime < timeoutMs) {
      try {
        const currentInterval = await this.queries.getCurrentInterval();
        const state = await this.queries.getTimelockState();
        console.log(
          `Current interval: ${currentInterval} (target: ${targetInterval}), last_rotation_time: ${state.last_rotation_time}`,
        );

        if (currentInterval >= targetInterval) {
          console.log(`✅ Interval rotation detected: ${currentInterval}`);
          return { current_interval: currentInterval };
        }
      } catch (error) {
        console.log(`Error checking interval: ${error}`);
      }
      await new Promise((resolve) => setTimeout(resolve, 1000)); // Check every 1 second
    }

    throw new Error(`Timeout waiting for interval rotation to ${targetInterval}`);
  }

  /**
   * Wait for public key publication
   */
  async waitForPublicKeyPublication(interval: number, timeoutSeconds: number = 60): Promise<Uint8Array> {
    const startTime = Date.now();
    const timeoutMs = timeoutSeconds * 1000;

    while (Date.now() - startTime < timeoutMs) {
      const transcript = await this.queries.verifyPublicKeyPublished(interval);
      if (transcript) {
        return transcript;
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    throw new Error(`Timeout waiting for public key publication for interval ${interval}`);
  }

  /**
   * Wait for secret aggregation
   */
  async waitForSecretAggregation(
    interval: number,
    threshold: number,
    timeoutSeconds: number = 60,
  ): Promise<Uint8Array> {
    const startTime = Date.now();
    const timeoutMs = timeoutSeconds * 1000;

    while (Date.now() - startTime < timeoutMs) {
      const secret = await this.queries.verifySecretAggregated(interval, threshold);
      if (secret) {
        return secret;
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    throw new Error(`Timeout waiting for secret aggregation for interval ${interval}`);
  }
}

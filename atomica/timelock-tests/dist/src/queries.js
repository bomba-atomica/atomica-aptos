/**
 * Helper functions for querying timelock blockchain state
 */
export class TimelockQueries {
    client;
    constructor(client) {
        this.client = client;
    }
    /**
     * Get current timelock interval
     */
    async getCurrentInterval() {
        const resources = await this.client.getAccountResources("0x1");
        const timelockResource = resources.find((r) => r.type.includes("timelock::TimelockState"));
        if (!timelockResource) {
            throw new Error("TimelockState resource not found");
        }
        return timelockResource.data.current_interval;
    }
    /**
     * Check if timelock is initialized
     */
    async isTimelockInitialized() {
        try {
            await this.getCurrentInterval();
            return true;
        }
        catch {
            return false;
        }
    }
    /**
     * Verify public key published for interval
     */
    async verifyPublicKeyPublished(interval) {
        try {
            const resources = await this.client.getAccountResources("0x1");
            const transcriptResource = resources.find((r) => r.type.includes("timelock::TranscriptStore") && r.data.interval === interval);
            if (transcriptResource) {
                return new Uint8Array(transcriptResource.data.transcript);
            }
        }
        catch (error) {
            // Resource not found or other error
        }
        return null;
    }
    /**
     * Verify secret aggregated for interval
     */
    async verifySecretAggregated(interval, threshold) {
        try {
            const resources = await this.client.getAccountResources("0x1");
            const secretResource = resources.find((r) => r.type.includes("timelock::DecryptionKeyStore") && r.data.interval === interval);
            if (secretResource && secretResource.data.share_count >= threshold) {
                return new Uint8Array(secretResource.data.decryption_key);
            }
        }
        catch (error) {
            // Resource not found or other error
        }
        return null;
    }
}
//# sourceMappingURL=queries.js.map
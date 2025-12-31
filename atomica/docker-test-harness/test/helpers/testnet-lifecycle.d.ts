import { DockerTestnet } from "../../src/index";
/**
 * Cleanup function that can be called from anywhere
 */
export declare function performCleanup(reason: string): Promise<void>;
/**
 * Register global cleanup handlers for the process
 */
export declare function registerCleanupHandlers(): void;
/**
 * Set the global testnet instance for cleanup handlers
 */
export declare function setGlobalTestnet(testnet: DockerTestnet | undefined): void;
/**
 * Get the global testnet instance
 */
export declare function getGlobalTestnet(): DockerTestnet | undefined;
/**
 * Initialize a testnet with the specified number of validators
 * and wait for consensus to start
 */
export declare function initializeTestnet(numValidators: number): Promise<DockerTestnet>;
/**
 * Wait for the network to stabilize and produce at least one block
 */
export declare function waitForNetworkStabilization(testnet: DockerTestnet): Promise<void>;
//# sourceMappingURL=testnet-lifecycle.d.ts.map
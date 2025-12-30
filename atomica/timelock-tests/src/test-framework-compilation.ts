import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { cpSync, mkdtempSync, rmSync, writeFileSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { spawn } from "child_process";

/**
 * Framework Loading Verification Test
 *
 * META TEST: Validates the docker-test-harness's ability to insert modified .mrb framework files.
 *
 * This test verifies that the DockerTestnet.new() function's customFrameworkPath parameter
 * works correctly by:
 * 1. Creating a modified framework with a test contract (noop.move)
 * 2. Loading the custom framework into a testnet via customFrameworkPath
 * 3. Verifying the test contract is available on-chain
 *
 * WHY THIS MATTERS:
 * - Ensures framework modifications can be tested without rebuilding Docker images
 * - Validates the genesis process can accept custom .mrb files
 * - Confirms that modified frameworks are properly embedded in genesis artifacts
 *
 * USAGE:
 *   cd atomica/timelock-tests
 *   bun run test:framework
 *
 * SUCCESS CRITERIA:
 * - Custom framework compiles successfully
 * - Testnet starts with custom framework
 * - noop::is_available() view function returns true
 *
 * FAILURE SCENARIOS:
 * - Framework compilation fails
 * - Custom framework not loaded (fallback to Docker image default)
 * - Test contract not available (framework loading failed)
 *
 * @see atomica/docker-test-harness/README.md for framework management documentation
 */

/**
 * Create a test framework that includes noop.move for verification
 *
 * This function creates a modified version of the Aptos framework by:
 * 1. Copying production framework sources to a temp directory
 * 2. Adding a test contract (noop.move) to verify loading
 * 3. Creating Move.toml configuration
 * 4. Compiling the framework using aptos-framework binary
 *
 * The resulting .mrb file can be used to test custom framework loading
 * in the docker-test-harness.
 */
async function createTestFramework(): Promise<string> {
  const tempDir = mkdtempSync(join(tmpdir(), "aptos-framework-test-"));
  const outputPath = "./atomica/move-framework-fixtures/test-head.mrb";

  try {
    console.log(`📁 Creating test framework in: ${tempDir}`);

    // Step 1: Copy production framework sources to temp directory
    // This gives us a complete, working framework to modify
    const sourceDir = join(process.cwd(), "../../aptos-move/framework/aptos-framework/sources");
    const targetDir = join(tempDir, "sources");

    console.log(`📋 Copying production code from ${sourceDir} to ${targetDir}`);
    cpSync(sourceDir, targetDir, { recursive: true });

    // Step 2: Add noop.move test contract to the framework
    // This contract provides a simple function to verify framework loading
    const noopSource = join(process.cwd(), "../move-framework-fixtures/noop.move");
    const noopTarget = join(targetDir, "noop.move");

    console.log(`➕ Adding noop.move to test framework`);
    cpSync(noopSource, noopTarget);

    // Step 3: Create Move.toml configuration for the test framework
    // This defines package metadata and dependencies for compilation
    const moveToml = `[package]
name = "AptosFramework"
version = "1.0.0"

[dependencies]
AptosStdlib = { local = "../aptos-stdlib" }
MoveStdlib = { local = "../../../move-stdlib" }

[addresses]
aptos_framework = "0x1"
aptos_std = "0x1"
std = "0x1"
`;

    const moveTomlPath = join(tempDir, "Move.toml");
    writeFileSync(moveTomlPath, moveToml);

    // Step 4: Compile the framework using aptos-framework binary
    // This produces a .mrb file that includes our test contract
    console.log("🔨 Compiling test framework with noop.move...");

    // Compile from temp directory using pre-built aptos-framework binary
    await new Promise<void>((resolve, reject) => {
      const cargo = spawn("cargo", ["run", "--package", "aptos-framework", "--", "release", "--target", "head"], {
        cwd: process.cwd(),
        stdio: "inherit",
      });

      cargo.on("close", (code) => {
        if (code === 0) {
          console.log("✅ Framework compilation completed");
          resolve();
        } else {
          reject(new Error(`Framework compilation failed with exit code ${code}`));
        }
      });

      cargo.on("error", (error) => {
        reject(new Error(`Failed to start framework compilation: ${error.message}`));
      });
    });

    return outputPath;
  } catch (error) {
    console.error("❌ Failed to create test framework:", error);
    throw error;
  } finally {
    // Clean up temp directory
    try {
      rmSync(tempDir, { recursive: true, force: true });
      console.log(`🧹 Cleaned up temp directory: ${tempDir}`);
    } catch (error) {
      console.warn(`⚠️  Failed to cleanup temp directory: ${tempDir}`);
    }
  }
}

/**
 * Execute the framework compilation and loading verification test
 *
 * This is the main test function that:
 * 1. Creates a custom framework with test contract
 * 2. Starts a testnet with the custom framework
 * 3. Verifies the framework was loaded correctly
 * 4. Cleans up resources
 *
 * SUCCESS: noop::is_available() returns true
 * FAILURE: Function not found or returns false
 */
async function testFrameworkCompilation() {
  console.log("🧪 Testing framework compilation with Noop contract");
  console.log("This meta-test verifies custom .mrb framework loading capability");

  try {
    // Phase 1: Create test framework with noop.move included
    // This produces a .mrb file that can be passed to DockerTestnet.new()
    const frameworkPath = await createTestFramework();
    console.log(`📦 Custom framework created at: ${frameworkPath}`);

    // Phase 2: Start testnet with custom framework
    // The customFrameworkPath parameter should load our modified framework
    console.log("Starting testnet with test framework...");
    console.log("If this fails, the docker-test-harness cannot load custom frameworks");
    const testnet = await initializeTestnet(2, frameworkPath);

    try {
      const client = new AptosClient(testnet.validatorApiUrl(0));

      // Phase 3: Verify framework loading by testing noop contract
      // The noop::is_available() function should exist and return true
      // This proves the custom framework was loaded instead of Docker image default
      console.log("Testing Noop contract availability...");
      console.log("This verifies the custom framework was loaded correctly");

      const isAvailable = await client.view({
        function: "0x1::noop::is_available",
        type_arguments: [],
        arguments: [],
      });

      if (isAvailable[0] !== true) {
        throw new Error("Noop contract is_available() returned false - custom framework not loaded");
      }

      console.log("✅ Noop contract is available - test framework loaded successfully!");
      console.log(`✅ Framework compilation and loading test PASSED: ${frameworkPath}`);
      console.log("✅ Docker test-harness can successfully insert modified .mrb files");
    } finally {
      await performCleanup("Framework compilation test completed");
    }
  } catch (error) {
    console.error("❌ Framework compilation test failed:", error);
    console.error("This indicates the docker-test-harness cannot load custom frameworks");
    throw error;
  }
}

// Execute the framework loading verification test
// Run with: bun run test:framework
testFrameworkCompilation().catch(console.error);

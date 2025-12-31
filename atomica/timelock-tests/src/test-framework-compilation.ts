import { initializeTestnet, performCleanup } from "../../docker-test-harness/test/helpers/testnet-lifecycle";
import { AptosClient } from "aptos";
import { cpSync, mkdtempSync, rmSync, writeFileSync, existsSync, readlinkSync } from "fs";
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
 * 1. Copying production framework to temp and adding noop.move
 * 2. Building the framework using build-framework.sh (content-addressable caching)
 * 3. Loading the custom framework into a testnet via customFrameworkPath
 * 4. Verifying the noop contract functions are available on-chain
 *
 * WHY THIS MATTERS:
 * - Ensures framework modifications can be tested without rebuilding Docker images
 * - Validates build-framework.sh produces working .mrb files with added contracts
 * - Confirms genesis process accepts custom .mrb files with content-addressable caching
 * - Verifies modified frameworks are properly embedded in genesis artifacts
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
 * 2. Adding noop.move to aptos-framework/sources (no Move.toml creation needed)
 * 3. Building the framework using build-framework.sh script
 *
 * The resulting .mrb file can be used to test custom framework loading
 * in the docker-test-harness.
 */
async function createTestFramework(): Promise<string> {
  const tempDir = mkdtempSync(join(tmpdir(), "aptos-framework-test-"));
  const frameworkDir = join(tempDir, "framework");
  const outputDir = join(tempDir, "output");

  try {
    console.log(`📁 Creating test framework in: ${tempDir}`);

    // Step 1: Copy production framework sources to temp directory
    // This gives us a complete, working framework to modify
    const sourceDir = join(process.cwd(), "../../aptos-move/framework");
    const targetDir = frameworkDir;

    console.log(`📋 Copying production framework from ${sourceDir} to ${targetDir}`);
    cpSync(sourceDir, targetDir, { recursive: true });

    // Step 2: Add noop.move to aptos-framework sources
    // This drops noop.move alongside the other framework move files
    const noopSource = join(process.cwd(), "../move-framework-fixtures/noop.move");
    const noopTarget = join(targetDir, "aptos-framework", "sources", "noop.move");

    console.log(`➕ Adding noop.move to aptos-framework sources`);
    cpSync(noopSource, noopTarget);

    // Step 3: Build the framework using build-framework.sh
    // This produces a .mrb file that includes our test contract
    console.log("🔨 Building test framework with noop.move using build-framework.sh...");

    await new Promise<void>((resolve, reject) => {
      const buildScript = join(process.cwd(), "../move-framework-fixtures/build-framework.sh");
      const buildProcess = spawn(buildScript, [frameworkDir, outputDir], {
        stdio: "inherit",
      });

      buildProcess.on("close", (code) => {
        if (code === 0) {
          console.log("✅ Framework build completed");
          resolve();
        } else {
          reject(new Error(`Framework build failed with exit code ${code}`));
        }
      });

      buildProcess.on("error", (error) => {
        reject(new Error(`Failed to start framework build: ${error.message}`));
      });
    });

    // The build-framework.sh creates head.mrb symlink pointing to head-{HASH}.mrb
    const symlinkPath = join(outputDir, "head.mrb");
    if (!existsSync(symlinkPath)) {
      throw new Error(`Expected symlink ${symlinkPath} to exist after build`);
    }

    // Copy the built framework (following symlink) to fixtures for the test
    const actualFile = readlinkSync(symlinkPath);
    const fixturesPath = join(process.cwd(), "../move-framework-fixtures/test-head.mrb");
    cpSync(join(outputDir, actualFile), fixturesPath);

    console.log(`📦 Test framework copied to: ${fixturesPath}`);
    return fixturesPath;
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
      });

      buildProcess.on("error", (error) => {
        reject(new Error(`Failed to start framework build: ${error.message}`));
      });
    });

    // The build-framework.sh creates head.mrb symlink pointing to head-{HASH}.mrb
    const symlinkPath = join(outputDir, "head.mrb");
    if (!existsSync(symlinkPath)) {
      throw new Error(`Expected symlink ${symlinkPath} to exist after build`);
    }

    // Copy the built framework (following symlink) to fixtures for the test
    const actualFile = readlinkSync(symlinkPath);
    const fixturesPath = join(process.cwd(), "../move-framework-fixtures/test-head.mrb");
    cpSync(join(outputDir, actualFile), fixturesPath);

    console.log(`📦 Test framework copied to: ${fixturesPath}`);
    return fixturesPath;
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
    // Phase 1: Create and build test framework with noop.move added
    // This produces a .mrb file with noop.move included alongside production contracts
    const frameworkPath = await createTestFramework();
    console.log(`📦 Test framework built at: ${frameworkPath}`);

    // Phase 2: Start testnet with custom framework
    // The customFrameworkPath parameter should load our modified framework
    console.log("🚀 Starting testnet with custom framework...");
    console.log("If this fails, the docker-test-harness cannot load custom .mrb files");
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

      console.log("✅ Noop contract is available - custom framework loaded successfully!");
      console.log(`✅ Framework build and loading test PASSED: ${frameworkPath}`);
      console.log("✅ Docker test-harness can successfully load custom .mrb files built with build-framework.sh");
    } finally {
      await performCleanup("Framework compilation test completed");
    }
  } catch (error) {
    console.error("❌ Framework build and loading test failed:", error);
    console.error("This indicates either build-framework.sh failed or docker-test-harness cannot load custom .mrb files");
    throw error;
  }
}

// Execute the framework loading verification test
// Run with: bun run test:framework
testFrameworkCompilation().catch(console.error);

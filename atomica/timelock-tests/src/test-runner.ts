#!/usr/bin/env bun

/**
 * Timelock Test Runner
 *
 * Executes timelock tests in different patterns:
 * - Sequential (round-robin): Run each test one by one
 * - Parallel (breadth-first): Run all tests simultaneously where possible
 */

import { spawn } from "child_process";
import { join } from "path";

const TEST_SCRIPTS = ["src/test-basic.ts", "src/test-ibe.ts"];

const TIMEOUT_MINUTES = 10; // 10 minutes per test

async function runTestSequential(testPath: string): Promise<{ success: boolean; duration: number }> {
  const startTime = Date.now();

  return new Promise((resolve) => {
    console.log(`\n▶️  Starting ${testPath}...`);

    const child = spawn("bun", ["run", testPath], {
      cwd: process.cwd(),
      stdio: "inherit",
      timeout: TIMEOUT_MINUTES * 60 * 1000,
    });

    child.on("close", (code) => {
      const duration = Date.now() - startTime;
      const success = code === 0;

      console.log(`\n${success ? "✅" : "❌"} ${testPath} completed in ${(duration / 1000).toFixed(1)}s`);
      resolve({ success, duration });
    });

    child.on("error", (error) => {
      const duration = Date.now() - startTime;
      console.log(`\n❌ ${testPath} failed with error: ${error.message}`);
      resolve({ success: false, duration });
    });
  });
}

async function runTestParallel(testPath: string): Promise<{ success: boolean; duration: number }> {
  // For parallel execution, we need to ensure tests don't conflict
  // For now, implement as sequential since docker testnets might conflict
  console.log(`⚠️  Parallel execution not yet implemented for ${testPath}, running sequentially`);
  return runTestSequential(testPath);
}

async function runRoundRobin() {
  console.log("🎯 Running tests in round-robin (sequential) pattern");
  console.log("=".repeat(60));

  const results = [];

  for (const test of TEST_SCRIPTS) {
    const result = await runTestSequential(test);
    results.push({ test, ...result });

    // Small delay between tests
    await new Promise((resolve) => setTimeout(resolve, 2000));
  }

  console.log("\n" + "=".repeat(60));
  console.log("🎯 Round-robin results:");
  results.forEach(({ test, success, duration }) => {
    console.log(`  ${success ? "✅" : "❌"} ${test}: ${(duration / 1000).toFixed(1)}s`);
  });

  const passed = results.filter((r) => r.success).length;
  const total = results.length;
  console.log(`\n📊 Summary: ${passed}/${total} tests passed`);

  return results.every((r) => r.success);
}

async function runBreadthFirst() {
  console.log("🎯 Running tests in breadth-first (parallel) pattern");
  console.log("⚠️  Currently implemented as sequential due to docker conflicts");
  console.log("=".repeat(60));

  // For now, run sequentially to avoid conflicts
  const results = [];

  for (const test of TEST_SCRIPTS) {
    const result = await runTestParallel(test);
    results.push({ test, ...result });

    // Small delay between tests
    await new Promise((resolve) => setTimeout(resolve, 2000));
  }

  console.log("\n" + "=".repeat(60));
  console.log("🎯 Breadth-first results:");
  results.forEach(({ test, success, duration }) => {
    console.log(`  ${success ? "✅" : "❌"} ${test}: ${(duration / 1000).toFixed(1)}s`);
  });

  const passed = results.filter((r) => r.success).length;
  const total = results.length;
  console.log(`\n📊 Summary: ${passed}/${total} tests passed`);

  return results.every((r) => r.success);
}

async function main() {
  const args = process.argv.slice(2);
  const pattern = args[0] || "round-robin";

  console.log("🚀 Timelock Test Runner");
  console.log(`Pattern: ${pattern}`);
  console.log(`Tests: ${TEST_SCRIPTS.join(", ")}`);
  console.log(`Timeout per test: ${TIMEOUT_MINUTES} minutes`);

  let success = false;

  try {
    if (pattern === "breadth-first") {
      success = await runBreadthFirst();
    } else {
      success = await runRoundRobin();
    }
  } catch (error) {
    console.error("❌ Test runner failed:", error);
    success = false;
  }

  console.log(`\n${success ? "🎉 All tests passed!" : "💥 Some tests failed"}`);
  process.exit(success ? 0 : 1);
}

main();

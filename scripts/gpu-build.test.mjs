import assert from "node:assert/strict";
import test from "node:test";

import {
  mergeCargoFeature,
  requestedTarget,
  usesVulkanTarget,
  vulkanPrerequisiteError,
} from "./gpu-build.mjs";

test("detects explicit targets and Vulkan platforms", () => {
  assert.equal(requestedTarget(["--target", "x86_64-unknown-linux-gnu"]), "x86_64-unknown-linux-gnu");
  assert.equal(requestedTarget(["--target=x86_64-pc-windows-msvc"]), "x86_64-pc-windows-msvc");
  assert.equal(usesVulkanTarget("aarch64-apple-darwin"), false);
  assert.equal(usesVulkanTarget("x86_64-unknown-linux-gnu"), true);
});

test("merges Vulkan into existing Cargo features without duplicates", () => {
  assert.deepEqual(mergeCargoFeature(["--debug"], "gpu-vulkan"), [
    "--features",
    "gpu-vulkan",
    "--debug",
  ]);
  assert.deepEqual(
    mergeCargoFeature(["--features", "custom,gpu-vulkan", "--target=linux"], "gpu-vulkan"),
    ["--features", "custom,gpu-vulkan", "--target=linux"],
  );
  assert.deepEqual(mergeCargoFeature(["--features=one", "--features", "two"], "gpu-vulkan"), [
    "--features",
    "one,two,gpu-vulkan",
  ]);
});

test("reports actionable Linux Vulkan prerequisites", () => {
  const runner = () => ({ status: 1 });
  const message = vulkanPrerequisiteError(
    "x86_64-unknown-linux-gnu",
    {},
    "linux",
    runner,
  );
  assert.match(message, /SPIRV-Headers/);
  assert.match(message, /sudo pacman -S/);
  assert.equal(vulkanPrerequisiteError("aarch64-apple-darwin", {}, "darwin", runner), undefined);
});

test("accepts a complete Windows Vulkan SDK", () => {
  const fileExists = (file) => file.endsWith("glslc.exe") || file.endsWith("vulkan.h");
  assert.equal(
    vulkanPrerequisiteError("x86_64-pc-windows-msvc", { VULKAN_SDK: "C:\\VulkanSDK" }, "win32", undefined, fileExists),
    undefined,
  );
});

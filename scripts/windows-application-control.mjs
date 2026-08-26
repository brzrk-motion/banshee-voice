import { spawnSync } from "node:child_process";

const POLICY_KEY = "HKLM\\SYSTEM\\CurrentControlSet\\Control\\CI\\Policy";
const POLICY_VALUE = "VerifiedAndReputablePolicyState";

export function parseSmartAppControlState(output) {
  const value = output.match(
    /VerifiedAndReputablePolicyState\s+REG_DWORD\s+(0x[0-9a-f]+|\d+)/i,
  )?.[1];
  if (!value) return undefined;
  return Number.parseInt(value, value.toLowerCase().startsWith("0x") ? 16 : 10);
}

export function classifySmartAppControlQuery(result) {
  if (result.error || result.status !== 0) return "unavailable";
  const state = parseSmartAppControlState(result.stdout ?? "");
  if (state === 1) return "enforced";
  if (state === 0 || state === 2) return "allowed";
  return "unavailable";
}

export function assertWindowsDevelopmentAllowed(environment = process.env) {
  if (process.platform !== "win32") return;

  const result = spawnSync(
    "reg.exe",
    ["query", POLICY_KEY, "/v", POLICY_VALUE],
    { encoding: "utf8", env: environment, windowsHide: true },
  );
  const classification = classifySmartAppControlQuery(result);
  if (classification === "allowed") return;
  if (classification === "unavailable") {
    console.warn(
      "Warning: Could not determine the Windows Smart App Control state; continuing the build.",
    );
    return;
  }

  throw new Error(
    [
      "Banshee's Rust tooling cannot run while Windows Smart App Control is On.",
      "It blocks Cargo-generated unsigned executables and DLLs, including the llama.cpp build script and prompt worker.",
      "Open Windows Security > App & browser control > Smart App Control settings, set it to Off, reboot Windows, then rerun `npm run tauri:dev`.",
      "Developer Mode alone does not disable a Smart App Control policy that is already enforcing.",
      "Alternatively, build in a Windows VM or machine where Smart App Control is not enforcing.",
    ].join("\n"),
  );
}

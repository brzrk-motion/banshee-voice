import { invoke } from "@tauri-apps/api/core";

export function run<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args);
}

export function errorMessage(error: unknown) {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return "Something went wrong.";
}

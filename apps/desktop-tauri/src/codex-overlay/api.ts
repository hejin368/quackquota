import { invoke } from "@tauri-apps/api/core";
import type {
  CodexProxyStatus,
  CodexProxyTestResult,
  CodexQuotaSnapshot,
} from "./types";

/** Window label shared by App routing and the detached overlay feature. */
export const CODEX_OVERLAY_WINDOW_LABEL = "codex-overlay";

/** Read the minimal Codex-only quota DTO from the dedicated Rust command. */
export function readCodexRateLimits(): Promise<CodexQuotaSnapshot> {
  return invoke<CodexQuotaSnapshot>("read_codex_rate_limits");
}

export function hideCodexOverlay(): Promise<void> {
  return invoke<void>("hide_codex_overlay");
}

export function resetCodexOverlayPosition(): Promise<void> {
  return invoke<void>("reset_codex_overlay_position");
}

export function getCodexProxyStatus(): Promise<CodexProxyStatus> {
  return invoke<CodexProxyStatus>("get_codex_proxy_status");
}

export function testCodexProxyConnection(): Promise<CodexProxyTestResult> {
  return invoke<CodexProxyTestResult>("test_codex_proxy_connection");
}

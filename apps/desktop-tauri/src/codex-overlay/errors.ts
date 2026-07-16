import type { LocaleKey } from "../i18n/keys";
import type { CodexConnectionErrorKind } from "./types";

export function connectionErrorLocaleKey(
  kind?: CodexConnectionErrorKind,
): LocaleKey | null {
  switch (kind) {
    case "dns-or-network":
      return "CodexProxyErrorDnsOrNetwork";
    case "proxy-connection":
      return "CodexProxyErrorProxyConnection";
    case "proxy-configuration":
      return "CodexProxyErrorConfiguration";
    case "not-logged-in":
      return "CodexProxyErrorNotLoggedIn";
    case "app-server-unavailable":
      return "CodexProxyErrorAppServerUnavailable";
    case "service-error":
      return "CodexProxyErrorService";
    case "invalid-data":
      return "CodexProxyErrorInvalidData";
    default:
      return null;
  }
}

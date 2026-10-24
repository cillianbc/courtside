// Errors in the window go to the backend's log, and to Sentry when the
// scout agreed (`diagnostics.rs`), with the actions before them.

import * as backend from "./backend";
import { recentActions } from "./trail";

export function catchWindowErrors() {
  const send = (message: string, stack: string) =>
    void backend.reportJsError(message, stack, recentActions()).catch(() => {});
  window.addEventListener("error", (e) => {
    send(String(e.message), e.error instanceof Error ? (e.error.stack ?? "") : "");
  });
  window.addEventListener("unhandledrejection", (e) => {
    const r: unknown = e.reason;
    send(r instanceof Error ? r.message : String(r), r instanceof Error ? (r.stack ?? "") : "");
  });
}

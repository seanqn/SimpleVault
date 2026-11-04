import { test as base, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import path from "node:path";
type Harness = {
  failNext: (type: string) => void;
  failRefreshAfter: (type: string) => void;
  delayNext: (type: string, ms: number) => void;
};
export const test = base.extend<{ harness: Harness }>({
  harness: [
    async ({ page }, use) => {
      const child = spawn(
        process.env.VAULT_HARNESS ||
          path.resolve("../target/debug/browser-harness"),
        [],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      const waiting: Array<{
        resolve: (value: any) => void;
        reject: (reason: Error) => void;
      }> = [];
      let failure: string | undefined;
      let refreshFailure: string | undefined;
      let delay: { type: string; ms: number } | undefined;
      const lines = createInterface({ input: child.stdout });
      lines.on("line", (line) => waiting.shift()?.resolve(JSON.parse(line)));
      child.on("error", (e) => waiting.splice(0).forEach((p) => p.reject(e)));
      child.on("exit", () =>
        waiting
          .splice(0)
          .forEach((p) => p.reject(new Error("Rust backend exited"))),
      );
      await page.exposeFunction(
        "testVaultInvoke",
        async (request: { type: string }) => {
          if (failure === request.type) {
            failure = undefined;
            return {
              Err: {
                code: "operation_failed",
                message:
                  "The change could not be completed. Your draft has been kept; try again.",
              },
            };
          }
          if (delay?.type === request.type) {
            const ms = delay.ms;
            delay = undefined;
            await new Promise((r) => setTimeout(r, ms));
          }
          const response: any = await new Promise((resolve, reject) => {
            waiting.push({ resolve, reject });
            child.stdin.write(JSON.stringify(request) + "\n");
          });
          if (refreshFailure === request.type && response.Ok) {
            refreshFailure = undefined;
            response.Ok.snapshot = null;
            response.Ok.refresh_error = {
              code: "operation_failed",
              message: "The view could not refresh.",
            };
          }
          return response;
        },
      );
      await page.addInitScript(() => {
        const w = window as any;
        w.isTauri = true;
        let next = 0;
        const callbacks = new Map<number, Function>();
        const listeners = new Map<number, number>();
        w.__TAURI_INTERNALS__ = {
          transformCallback: (fn: Function) => {
            callbacks.set(++next, fn);
            return next;
          },
          unregisterCallback: (id: number) => callbacks.delete(id),
          invoke: async (command: string, args: any) => {
            if (command === "plugin:event|listen") {
              listeners.set(args.handler, args.handler);
              return args.handler;
            }
            if (command === "plugin:event|unlisten") {
              listeners.delete(args.eventId);
              return;
            }
            const response = await w.testVaultInvoke(args.request);
            if (response.Err) throw response.Err;
            return response.Ok;
          },
        };
        w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
          unregisterListener: (_event: string, id: number) =>
            listeners.delete(id),
        };
        w.testNativeClose = () =>
          listeners.forEach((id) =>
            callbacks.get(id)?.({
              event: "vault-close-requested",
              payload: null,
            }),
          );
      });
      await use({
        failNext: (type) => (failure = type),
        failRefreshAfter: (type) => (refreshFailure = type),
        delayNext: (type, ms) => (delay = { type, ms }),
      });
      child.stdin.end();
      child.kill();
      lines.close();
    },
    { auto: true },
  ],
});
export { expect };

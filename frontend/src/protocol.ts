import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Group {
  id: string;
  name: string;
}
export interface Credential {
  id: string;
  org_name: string;
  username: string;
  email: string;
  note: string;
}
export type Resolution = "save" | "discard" | "cancel";
export interface Snapshot {
  session: "locked" | "unlocked" | "closed";
  temporary: boolean;
  uid: string | null;
  groups: Group[];
  credentials: Credential[];
  selected_group: string | null;
  selected_credential: string | null;
  draft: Credential | null;
  dirty: boolean;
}
export interface EditFields {
  org_name: string;
  username: string;
  email: string;
  note: string;
  password?: string;
}
export type Request =
  | { type: "create_vault"; password: string }
  | { type: "unlock_vault"; uid: string; password: string }
  | {
      type:
        | "snapshot"
        | "unlock"
        | "add_credential"
        | "submit_edit"
        | "cancel_edit";
    }
  | { type: "create_group"; name: string }
  | { type: "rename_group"; id: string; name: string }
  | { type: "select_group"; id: string | null }
  | {
      type:
        "remove_group" | "begin_edit" | "remove_credential" | "reveal_password";
      id: string;
    }
  | { type: "update_draft"; fields: EditFields }
  | { type: "lock" | "shutdown"; resolution: Resolution };
export interface ClientError {
  code: string;
  message: string;
}
export interface Response {
  snapshot: Snapshot | null;
  refresh_error: ClientError | null;
  secret?: string;
}
export const desktopAvailable = () => isTauri();
export const request = (request: Request) =>
  invoke<Response>("vault_command", { request });
export function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return "The operation could not be completed. Your draft has been kept; try again.";
}

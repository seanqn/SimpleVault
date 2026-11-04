import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type FormEvent,
} from "react";
import { listen } from "@tauri-apps/api/event";
import {
  desktopAvailable,
  errorMessage,
  request,
  type EditFields,
  type Request,
  type Snapshot,
} from "./protocol";

type DialogState =
  | { kind: "create" }
  | { kind: "rename"; id: string; name: string }
  | { kind: "delete-group"; id: string; name: string }
  | { kind: "delete-credential"; id: string; name: string }
  | { kind: "exit"; target: "lock" | "shutdown" };
const emptyFields = (): EditFields => ({
  org_name: "",
  username: "",
  email: "",
  note: "",
});
function LockIcon() {
  return (
    <svg
      width="28"
      height="28"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <rect x="5" y="10" width="14" height="11" rx="3" />
      <path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3" />
    </svg>
  );
}
export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const snapshotRef = useRef<Snapshot | null>(null);
  const [fields, setFields] = useState<EditFields>(emptyFields);
  const fieldsRef = useRef(fields);
  const editorId = useRef<string | null>(null);
  const [localDirty, setLocalDirty] = useState(false);
  const dirtyRef = useRef(false);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const [error, setError] = useState("");
  const [needsRefresh, setNeedsRefresh] = useState(false);
  const [revealed, setRevealed] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [groupName, setGroupName] = useState("");
  const dialogElement = useRef<HTMLDialogElement>(null);
  const available = desktopAvailable();
  const setEditFields = (value: EditFields) => {
    fieldsRef.current = value;
    setFields(value);
  };
  const apply = useCallback((next: Snapshot) => {
    snapshotRef.current = next;
    setSnapshot(next);
    setNeedsRefresh(false);
    const id = next.draft?.id ?? null;
    if (id !== editorId.current || next.session !== "unlocked") {
      editorId.current = id;
      const value = next.draft
        ? {
            org_name: next.draft.org_name,
            username: next.draft.username,
            email: next.draft.email,
            note: next.draft.note,
          }
        : emptyFields();
      fieldsRef.current = value;
      setFields(value);
      dirtyRef.current = false;
      setLocalDirty(false);
      setRevealed(null);
    }
    if (next.session !== "unlocked") {
      setQuery("");
      setDialog(null);
      setRevealed(null);
    }
  }, []);
  async function send(action: Request) {
    const response = await request(action);
    if (response.snapshot) apply(response.snapshot);
    if (response.refresh_error) {
      setNeedsRefresh(true);
      setError(
        "The change completed, but the view could not refresh. Refresh before continuing.",
      );
    }
    return response;
  }
  async function perform(
    action: Request,
    flushFirst = false,
  ): Promise<boolean> {
    if (busyRef.current) return false;
    busyRef.current = true;
    setBusy(true);
    setError("");
    setRevealed(null);
    try {
      if (flushFirst && dirtyRef.current && snapshotRef.current?.draft) {
        const updated = await send({
          type: "update_draft",
          fields: fieldsRef.current,
        });
        dirtyRef.current = false;
        setLocalDirty(false);
        setEditFields({ ...fieldsRef.current, password: undefined });
        if (updated.refresh_error) return false;
      }
      const response = await send(action);
      if (action.type === "reveal_password") setRevealed(response.secret ?? "");
      if (action.type === "select_group") setQuery("");
      return true; // Mutation succeeded even if a subsequent display refresh failed.
    } catch (e) {
      setError(errorMessage(e));
      return false;
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }
  useEffect(() => {
    if (available) void perform({ type: "snapshot" });
  }, []);
  useEffect(() => {
    if (revealed === null) return;
    const hide = () => setRevealed(null);
    const timer = setTimeout(hide, 15000);
    window.addEventListener("blur", hide);
    document.addEventListener("visibilitychange", hide);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("blur", hide);
      document.removeEventListener("visibilitychange", hide);
    };
  }, [revealed]);
  useEffect(() => {
    if (!available) return;
    let disposed = false;
    let remove: (() => void) | undefined;
    void listen("vault-close-requested", () =>
      setDialog({ kind: "exit", target: "shutdown" }),
    )
      .then((unlisten) => {
        if (disposed) unlisten();
        else remove = unlisten;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      remove?.();
    };
  }, [available]);
  useEffect(() => {
    const node = dialogElement.current;
    if (dialog && !node?.open) node?.showModal();
    else if (!dialog && node?.open) node.close();
  }, [dialog]);
  function change(field: keyof EditFields, value: string) {
    setEditFields({ ...fieldsRef.current, [field]: value });
    dirtyRef.current = true;
    setLocalDirty(true);
    setRevealed(null);
  }
  function openDialog(value: DialogState) {
    setError("");
    setGroupName(value.kind === "rename" ? value.name : "");
    setDialog(value);
  }
  function exit(target: "lock" | "shutdown") {
    if (localDirty || snapshot?.dirty) openDialog({ kind: "exit", target });
    else void perform({ type: target, resolution: "save" }, true);
  }
  async function confirmDialog(event: FormEvent) {
    event.preventDefault();
    if (!dialog) return;
    let ok = false;
    switch (dialog.kind) {
      case "create":
        ok = await perform({ type: "create_group", name: groupName });
        break;
      case "rename":
        ok = await perform({
          type: "rename_group",
          id: dialog.id,
          name: groupName,
        });
        break;
      case "delete-group":
        ok = await perform({ type: "remove_group", id: dialog.id });
        break;
      case "delete-credential":
        ok = await perform({ type: "remove_credential", id: dialog.id });
        break;
      default:
        return;
    }
    if (ok) setDialog(null);
  }
  const disabled = busy || needsRefresh;
  const selectedGroup = snapshot?.groups.find(
    (g) => g.id === snapshot.selected_group,
  );
  const rows =
    snapshot?.credentials.filter((row) =>
      [row.org_name, row.username, row.email].some((value) =>
        value.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
      ),
    ) ?? [];
  const locked = snapshot?.session !== "unlocked";
  return (
    <main className={locked ? "auth-window" : "vault-window"} aria-busy={busy}>
      {!available ? (
        <section className="auth-card">
          <LockIcon />
          <h1>SimpleVault</h1>
          <p>Open SimpleVault in the desktop app to access your vault.</p>
        </section>
      ) : !snapshot ? (
        <section className="auth-card">
          <LockIcon />
          <h1>SimpleVault</h1>
          <p>
            {busy ? "Opening your vault…" : "Your vault could not be opened."}
          </p>
          {!busy && (
            <button onClick={() => void perform({ type: "snapshot" })}>
              Try again
            </button>
          )}
        </section>
      ) : snapshot.session === "closed" ? (
        <section className="auth-card">
          <LockIcon />
          <h1>Vault closed</h1>
          <p>Your session has ended.</p>
        </section>
      ) : snapshot.session === "locked" ? (
        <>
          <section className="auth-card" aria-labelledby="unlock-title">
            <div className="lock-badge">
              <LockIcon />
            </div>
            <span className="eyebrow">PRIVATE. LOCAL. YOURS.</span>
            <h1 id="unlock-title">SimpleVault</h1>
            <p>
              {snapshot.temporary
                ? "A quiet place to try your vault. This session is temporary."
                : "Your credentials, securely kept on this device."}
            </p>
            <button
              className="primary"
              disabled={busy}
              onClick={() => void perform({ type: "unlock" })}
            >
              {busy ? "Unlocking…" : "Unlock Vault"}
            </button>
            <small>
              {snapshot.temporary
                ? "Cleared when you quit"
                : "Authenticate with your device"}
            </small>
          </section>
          <footer className="auth-footer">
            <span>UID: {snapshot.uid ?? "—"}</span>
            <span>Local storage</span>
          </footer>
        </>
      ) : (
        <section className="vault-shell" aria-label="Vault">
          <header className="vault-header" data-tauri-drag-region>
            <div className="brand" data-tauri-drag-region>
              <LockIcon />
              <strong>SimpleVault</strong>
            </div>
            <div className="header-actions">
              <button
                className="icon-button"
                title="Lock vault"
                aria-label="Lock vault"
                disabled={disabled}
                onClick={() => exit("lock")}
              >
                ⌑
              </button>
              <button
                className="icon-button"
                title="Quit SimpleVault"
                aria-label="Quit SimpleVault"
                disabled={disabled}
                onClick={() => exit("shutdown")}
              >
                ×
              </button>
            </div>
          </header>
          {snapshot.temporary && (
            <div className="session-notice">
              Temporary vault · cleared when you quit
            </div>
          )}
          {snapshot.draft ? (
            <>
              <div className="editor-heading">
                <button
                  className="text-button"
                  disabled={disabled}
                  onClick={() => void perform({ type: "cancel_edit" })}
                >
                  ← Cancel
                </button>
                <span>
                  {localDirty || snapshot.dirty
                    ? "Unsaved changes"
                    : "Edit credential"}
                </span>
              </div>
              <form
                className="editor"
                onSubmit={(e) => {
                  e.preventDefault();
                  void perform({ type: "submit_edit" }, true);
                }}
              >
                <div className="editor-fields">
                  <label>
                    Organization
                    <input
                      autoFocus
                      name="organization"
                      autoComplete="off"
                      value={fields.org_name}
                      disabled={disabled}
                      onChange={(e) => change("org_name", e.target.value)}
                      placeholder="e.g. GitHub"
                    />
                  </label>
                  <label>
                    Username
                    <input
                      name="username"
                      autoComplete="off"
                      value={fields.username}
                      disabled={disabled}
                      onChange={(e) => change("username", e.target.value)}
                      placeholder="Username"
                    />
                  </label>
                  <label>
                    Email
                    <input
                      name="email"
                      inputMode="email"
                      autoComplete="off"
                      value={fields.email}
                      disabled={disabled}
                      onChange={(e) => change("email", e.target.value)}
                      placeholder="you@example.com"
                    />
                  </label>
                  <div className="password-label">
                    <label htmlFor="credential-password">Password</label>
                    <div className="password-field">
                      <input
                        id="credential-password"
                        type="password"
                        name="password"
                        autoComplete="new-password"
                        value={fields.password ?? ""}
                        disabled={disabled}
                        onChange={(e) => change("password", e.target.value)}
                        placeholder="Leave unchanged"
                      />
                      <button
                        type="button"
                        className="text-button"
                        disabled={disabled}
                        onClick={() =>
                          revealed !== null
                            ? setRevealed(null)
                            : void perform(
                                {
                                  type: "reveal_password",
                                  id: snapshot.draft!.id,
                                },
                                true,
                              )
                        }
                      >
                        {revealed === null ? "Reveal" : "Hide"}
                      </button>
                    </div>
                  </div>
                  {revealed !== null && (
                    <output className="revealed" aria-label="Revealed password">
                      {revealed || "(empty)"}
                    </output>
                  )}
                  <button
                    type="button"
                    className="text-button clear-password"
                    disabled={disabled}
                    onClick={() => change("password", "")}
                  >
                    Clear password
                  </button>
                  <label>
                    Notes
                    <textarea
                      name="note"
                      value={fields.note}
                      disabled={disabled}
                      onChange={(e) => change("note", e.target.value)}
                      placeholder="Anything else to remember"
                      rows={3}
                    />
                  </label>
                </div>
                <footer className="editor-footer">
                  <button
                    type="button"
                    className="danger text-button"
                    disabled={disabled}
                    onClick={() =>
                      openDialog({
                        kind: "delete-credential",
                        id: snapshot.draft!.id,
                        name: fields.org_name || "this credential",
                      })
                    }
                  >
                    Delete
                  </button>
                  <button type="submit" className="primary" disabled={disabled}>
                    {busy ? "Saving…" : "Save credential"}
                  </button>
                </footer>
              </form>
            </>
          ) : (
            <>
              <div className="group-bar">
                <label className="sr-only" htmlFor="group-picker">
                  Group
                </label>
                <select
                  id="group-picker"
                  aria-label="Group"
                  value={snapshot.selected_group ?? ""}
                  disabled={disabled || snapshot.groups.length === 0}
                  onChange={(e) =>
                    void perform(
                      { type: "select_group", id: e.target.value || null },
                      true,
                    )
                  }
                >
                  <option value="">
                    {snapshot.groups.length ? "Choose a group" : "Your groups"}
                  </option>
                  {snapshot.groups.map((group) => (
                    <option key={group.id} value={group.id}>
                      {group.name}
                    </option>
                  ))}
                </select>
                <button
                  className="icon-button add-group"
                  title="Add group"
                  aria-label="Add group"
                  disabled={disabled}
                  onClick={() => openDialog({ kind: "create" })}
                >
                  +
                </button>
              </div>
              {selectedGroup && (
                <div className="group-options">
                  <span>
                    {snapshot.credentials.length}{" "}
                    {snapshot.credentials.length === 1
                      ? "credential"
                      : "credentials"}
                  </span>
                  <button
                    className="text-button"
                    disabled={disabled}
                    onClick={() =>
                      openDialog({
                        kind: "rename",
                        id: selectedGroup.id,
                        name: selectedGroup.name,
                      })
                    }
                  >
                    Rename
                  </button>
                  <button
                    className="text-button"
                    disabled={disabled}
                    onClick={() =>
                      openDialog({
                        kind: "delete-group",
                        id: selectedGroup.id,
                        name: selectedGroup.name,
                      })
                    }
                  >
                    Delete group
                  </button>
                </div>
              )}
              {selectedGroup && (
                <label className="search">
                  <span aria-hidden="true">⌕</span>
                  <input
                    aria-label="Search credentials"
                    placeholder="Search credentials"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    disabled={disabled}
                  />
                  {query && (
                    <button
                      className="icon-button"
                      aria-label="Clear search"
                      onClick={() => setQuery("")}
                    >
                      ×
                    </button>
                  )}
                </label>
              )}
              <div className="credential-list">
                {!selectedGroup ? (
                  <div className="empty-state">
                    <div className="empty-symbol">▤</div>
                    <h2>
                      {snapshot.groups.length
                        ? "Choose your group"
                        : "A little more organized"}
                    </h2>
                    <p>
                      {snapshot.groups.length
                        ? "Select a group to see its credentials."
                        : "Create a group to keep your credentials together."}
                    </p>
                    {!snapshot.groups.length && (
                      <button
                        className="secondary"
                        disabled={disabled}
                        onClick={() => openDialog({ kind: "create" })}
                      >
                        Create your first group
                      </button>
                    )}
                  </div>
                ) : rows.length ? (
                  rows.map((row) => (
                    <button
                      className="credential-row"
                      key={row.id}
                      disabled={disabled}
                      onClick={() =>
                        void perform({ type: "begin_edit", id: row.id }, true)
                      }
                    >
                      <span className="credential-monogram">
                        {(row.org_name || row.username || "?")
                          .slice(0, 1)
                          .toLocaleUpperCase()}
                      </span>
                      <span className="credential-copy">
                        <strong>{row.org_name || "Untitled credential"}</strong>
                        <small>
                          {row.username || row.email || "No username"}
                        </small>
                      </span>
                      <span className="row-chevron" aria-hidden="true">
                        ›
                      </span>
                    </button>
                  ))
                ) : (
                  <div className="empty-state">
                    <div className="empty-symbol">◇</div>
                    <h2>{query ? "No matches" : "Ready when you are"}</h2>
                    <p>
                      {query
                        ? "Try another organization, username, or email."
                        : "Add the first credential to this group."}
                    </p>
                  </div>
                )}
              </div>
              <footer className="vault-footer">
                <span>
                  <i />
                  Stored on this device
                </span>
                <button
                  className="primary"
                  disabled={disabled || !selectedGroup}
                  onClick={() => void perform({ type: "add_credential" }, true)}
                >
                  + Add
                </button>
              </footer>
            </>
          )}
        </section>
      )}
      {(error || needsRefresh) && !dialog && (
        <div className="error-banner" role="alert">
          <span>{error || "Refresh the vault before continuing."}</span>
          {needsRefresh && (
            <button
              disabled={busy}
              onClick={() => void perform({ type: "snapshot" })}
            >
              Refresh
            </button>
          )}
        </div>
      )}
      <dialog
        ref={dialogElement}
        className="dialog"
        aria-labelledby="dialog-title"
        onCancel={(e) => {
          e.preventDefault();
          if (!busy) setDialog(null);
        }}
      >
        {dialog && (
          <form onSubmit={confirmDialog}>
            <h2 id="dialog-title">
              {dialog.kind === "create"
                ? "New group"
                : dialog.kind === "rename"
                  ? "Rename group"
                  : dialog.kind === "delete-group"
                    ? "Delete group?"
                    : dialog.kind === "delete-credential"
                      ? "Delete credential?"
                      : dialog.target === "lock"
                        ? "Lock your vault?"
                        : "Close SimpleVault?"}
            </h2>
            {dialog.kind === "create" || dialog.kind === "rename" ? (
              <label>
                Group name
                <input
                  autoFocus
                  value={groupName}
                  disabled={busy}
                  onChange={(e) => setGroupName(e.target.value)}
                  placeholder="e.g. Personal"
                  required
                />
              </label>
            ) : dialog.kind === "exit" ? (
              <p>
                Save your changes before{" "}
                {dialog.target === "lock" ? "locking" : "closing"}, or discard
                the current draft.
              </p>
            ) : (
              <p>
                Delete “{dialog.name}”
                {dialog.kind === "delete-group"
                  ? " and all its credentials"
                  : ""}
                ? This cannot be undone.
              </p>
            )}
            {error && (
              <p className="dialog-error" role="alert">
                {error}
              </p>
            )}
            {needsRefresh && (
              <button
                type="button"
                disabled={busy}
                onClick={() => void perform({ type: "snapshot" })}
              >
                Refresh
              </button>
            )}
            <div className="dialog-actions">
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setDialog(null);
                  setError("");
                }}
              >
                Cancel
              </button>
              {dialog.kind === "exit" ? (
                <>
                  <button
                    type="button"
                    className="danger"
                    disabled={busy}
                    onClick={() =>
                      void perform({
                        type: dialog.target,
                        resolution: "discard",
                      }).then((ok) => {
                        if (ok) setDialog(null);
                      })
                    }
                  >
                    Discard
                  </button>
                  <button
                    type="button"
                    className="primary"
                    disabled={busy}
                    onClick={() =>
                      void perform(
                        { type: dialog.target, resolution: "save" },
                        true,
                      ).then((ok) => {
                        if (ok) setDialog(null);
                      })
                    }
                  >
                    Save &amp; {dialog.target === "lock" ? "lock" : "quit"}
                  </button>
                </>
              ) : (
                <button
                  type="submit"
                  className={
                    dialog.kind.startsWith("delete") ? "danger" : "primary"
                  }
                  disabled={
                    disabled ||
                    ((dialog.kind === "create" || dialog.kind === "rename") &&
                      !groupName.trim())
                  }
                >
                  {dialog.kind === "create"
                    ? "Create group"
                    : dialog.kind === "rename"
                      ? "Save name"
                      : "Delete"}
                </button>
              )}
            </div>
          </form>
        )}
      </dialog>
    </main>
  );
}

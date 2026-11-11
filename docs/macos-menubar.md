# macOS menu-bar lifecycle

On macOS, both normal and development builds install a template status icon and run as an accessory application (no Dock icon). Left-click toggles the interface. Right-click offers Open SimpleVault and Quit SimpleVault. Quit uses the existing frontend save/discard/cancel flow; the top-left power button is also available during registration, authentication, and vault editing.

A new vault opens registration in the 700×600 window. After creation, the same webview becomes a 320×400 undecorated menu-bar popup. Existing vaults start hidden and open that popup for master-password authentication. Locking retains the popup dimensions. Clicking away hides the popup without locking, saving, or discarding an edit. Native window close hides the interface; explicit Quit shuts down the vault and terminates the process. Registration remains a regular window on focus loss.

The popup is anchored below the status icon with a four-point gap and horizontal monitor bounds checking. Native behavior is macOS-only; other platforms retain their existing window lifecycle.

## Native acceptance checks

Run `npm --prefix frontend run desktop:dev` on macOS, and also test `desktop:demo`. Verify first-run registration, creation-to-popup transition, restart and unlock, click-away dismissal, repeated icon toggling, lock/unlock sizing, and quit from both locked and unlocked states. Verify save/discard/cancel with a dirty draft. Check position on Retina and non-Retina screens, multiple monitors, and both menu-bar appearances. Browser tests exercise the controls and session commands but cannot validate AppKit focus, positioning, or application termination.

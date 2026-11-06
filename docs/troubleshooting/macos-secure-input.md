# macOS: shortcuts stopped working while Secure Input is active

macOS Secure Input protects password entry and can prevent global keyboard
shortcuts from reaching klava-nevinovata. This is separate from the app's
Accessibility permission.

1. Finish or close any password prompt. Focus a normal text field and try the
   shortcut again.
2. If Terminal has **Terminal → Secure Keyboard Entry** enabled, turn it off
   when you no longer need protected input.
3. If the warning names an application, finish protected input there. If it
   remains stuck, save your work and quit that application normally.
4. If the warning persists after those applications are closed, save your work
   before logging out of macOS and back in.

Some shortcuts can use a fallback while Secure Input is active. Side-specific
modifiers can become less precise; some combinations remain unavailable. The
warning reports when this affects your configured shortcuts.

For a missing Accessibility permission instead, see the
[permission recovery instructions](../../README.md#troubleshooting).

import assert from "node:assert/strict";
import { createServer } from "vite";

// Load the same bundled Markdown as the app, including Vite's import.meta.glob.
const server = await createServer({
  configFile: false,
  optimizeDeps: { noDiscovery: true },
  server: { middlewareMode: true, hmr: false, watch: null },
});

try {
  const { findReleaseNoteToShow } = await server.ssrLoadModule(
    "/src/components/whats-new/releaseNotes.ts",
  );
  const noteFor = (currentVersion, lastSeenVersion) =>
    findReleaseNoteToShow({ currentVersion, lastSeenVersion });

  for (const previous of ["0.10.1", "0.10.1-2"]) {
    const note = noteFor("0.10.1-3", previous);
    assert.equal(note?.version, "0.10.1-3");
    assert.match(note.markdown, /Breeze-ASR-25/);
  }
  assert.equal(noteFor("0.10.1-3", "0.10.1-3"), null);
  assert.equal(noteFor("0.10.1-2", "0.10.1"), null);
  assert.equal(noteFor("0.10.1-3", "0.10.1-10"), null);
  // A maintenance update keeps the last feature announcement available once.
  for (const previous of ["", "0.10.1-2"]) {
    const note = noteFor("0.10.2-1", previous);
    assert.equal(note?.version, "0.10.1-3");
    assert.match(note.markdown, /Breeze-ASR-25/);
    assert.match(note.markdown, /V3 — Ru-En/);
  }
  assert.equal(noteFor("0.10.2-1", "0.10.1-3"), null);
  // Fresh profiles initialize the last-seen version to the current app version.
  assert.equal(noteFor("0.10.2-1", "0.10.2-1"), null);
  console.log("Release note selection: OK");
} finally {
  await server.close();
}

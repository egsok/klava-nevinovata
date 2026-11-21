import { commands, type Theme } from "@/bindings";

/**
 * Appearance theme handling.
 *
 * Handy already ships a full light palette and a full dark palette (see
 * `App.css`). This module lets the user pick which one is used instead of
 * always following the OS:
 *  - `system` removes the override so the `prefers-color-scheme` media query
 *    governs (the historical behaviour).
 *  - `light` / `dark` set `data-theme` on the document root, whose
 *    higher-specificity CSS selectors win over the media query.
 *
 * The choice is persisted in `AppSettings` (source of truth) and mirrored to
 * localStorage so it can be applied synchronously on boot, before React mounts,
 * avoiding a flash of the wrong palette.
 */

export const THEME_STORAGE_KEY = "handy.theme";
const INITIAL_THEME: Theme = "light";

export const THEME_OPTIONS: Theme[] = ["system", "light", "dark"];

const isTheme = (value: unknown): value is Theme =>
  value === "system" || value === "light" || value === "dark";

/** Does this theme paint the ink wall right now? `system` asks the OS. */
const resolvesDark = (theme: Theme): boolean =>
  theme === "dark" ||
  (theme === "system" &&
    window.matchMedia?.("(prefers-color-scheme: dark)").matches === true);

/**
 * Tint the native window frame to match the palette. The frame is drawn by the
 * OS outside the webview, so CSS cannot reach it — only the backend can, and
 * only the webview knows what `system` currently resolves to. Windows 11 only;
 * a no-op on every other platform.
 */
const applyTitlebar = (theme: Theme): void => {
  void commands.setTitlebarTheme(resolvesDark(theme)).catch((e) => {
    console.warn("Failed to tint the native title bar:", e);
  });
};

/** Apply a theme to the document root and remember it for the next launch. */
export const applyTheme = (theme: Theme): void => {
  const root = document.documentElement;
  if (theme === "system") {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = theme;
  }
  applyTitlebar(theme);
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // localStorage may be unavailable (e.g. private mode); the setting still
    // persists in AppSettings, so this only costs a one-frame flash on boot.
  }
};

// Under `system`, CSS re-paints itself when the OS flips, but the native frame
// only changes if we tell the backend — so follow the OS here too.
window
  .matchMedia?.("(prefers-color-scheme: dark)")
  .addEventListener("change", () => {
    if (getStoredTheme() === "system") applyTitlebar("system");
  });

/** Read the last-applied theme for synchronous boot-time application. */
export const getStoredTheme = (): Theme => {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (isTheme(stored)) return stored;
  } catch {
    // ignore
  }
  return INITIAL_THEME;
};

/** Apply the persisted theme from AppSettings (the source of truth). */
export const syncThemeFromSettings = async (): Promise<void> => {
  try {
    const result = await commands.getAppSettings();
    if (result.status === "ok") {
      applyTheme(result.data.theme ?? INITIAL_THEME);
    }
  } catch (e) {
    console.warn("Failed to sync theme from settings:", e);
  }
};

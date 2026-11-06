import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { platform } from "@tauri-apps/plugin-os";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
  requestAccessibilityPermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { notify } from "@/stores/noticeStore";
import { commands } from "@/bindings";
import { useSettingsStore } from "@/stores/settingsStore";

type PermissionStatus = "checking" | "needed" | "waiting" | "granted";
type PermissionPlatform = "macos" | "windows" | "other";

export interface Permissions {
  platform: PermissionPlatform;
  microphone: PermissionStatus;
  accessibility: PermissionStatus;
  /** The first check is done. */
  checked: boolean;
  /** Something still needs allowing (decided by the first check). */
  needed: boolean;
  allGranted: boolean;
  /** Asked once already: asking again opens the OS settings instead. */
  microphoneAsked: boolean;
  requestMicrophone: () => Promise<void>;
  requestAccessibility: () => Promise<void>;
  resetAccessibility: () => Promise<void>;
  resetBusy: boolean;
  resetDone: boolean;
  restartRequired: boolean;
}

const MAX_POLLING_ERRORS = 3;
// No answer seen this long after asking: offer the button again.
const WAITING_TIMEOUT_MS = 15_000;

/** macOS's answers: `[accessibility, microphone]`. */
const readMac = () =>
  Promise.all([checkAccessibilityPermission(), checkMicrophonePermission()]);

const currentPlatform = (): PermissionPlatform => {
  const p = platform();
  return p === "macos" ? "macos" : p === "windows" ? "windows" : "other";
};

/**
 * Microphone (macOS, Windows) and Accessibility (macOS) permissions: check,
 * request, then poll until granted. Granting Accessibility initializes input
 * so no relaunch is needed.
 */
export const usePermissions = (preview: boolean): Permissions => {
  const { t } = useTranslation();
  const refreshAudioDevices = useSettingsStore((s) => s.refreshAudioDevices);
  const refreshOutputDevices = useSettingsStore((s) => s.refreshOutputDevices);
  const [os] = useState(currentPlatform);
  const [microphone, setMicrophone] = useState<PermissionStatus>("checking");
  const [accessibility, setAccessibility] =
    useState<PermissionStatus>("checking");
  const [needed, setNeeded] = useState(false);
  const [microphoneAsked, setMicrophoneAsked] = useState(false);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const errorCountRef = useRef(0);
  const generationRef = useRef(0);
  const resetBusyRef = useRef(false);
  const resetEverSucceededRef = useRef(false);
  const [resetBusy, setResetBusy] = useState(false);
  const [resetDone, setResetDone] = useState(false);
  const [restartRequired, setRestartRequired] = useState(false);
  const canApply = (generation: number) =>
    generation === generationRef.current &&
    !resetBusyRef.current &&
    !resetEverSucceededRef.current;

  const isMacOS = os === "macos";
  const isWindows = os === "windows";
  const allGranted =
    !resetBusy &&
    !restartRequired &&
    (isMacOS
      ? accessibility === "granted" && microphone === "granted"
      : isWindows
        ? microphone === "granted"
        : true);

  const hasWindowsMicrophoneAccess = useCallback(async () => {
    const status = await commands.getWindowsMicrophonePermissionStatus();
    return !status.supported || status.overall_access !== "denied";
  }, []);

  const initializeAfterAccessibility = () =>
    Promise.all([
      commands.initializeEnigo(),
      commands.initializeShortcuts(),
    ]).catch((e) => {
      console.warn("Failed to initialize after permission grant:", e);
    });

  const stopPolling = () => {
    if (pollingRef.current) {
      clearInterval(pollingRef.current);
      pollingRef.current = null;
    }
  };

  const onAllGranted = useCallback(async () => {
    await Promise.all([refreshAudioDevices(), refreshOutputDevices()]);
  }, [refreshAudioDevices, refreshOutputDevices]);

  // The first check: decides whether the Permissions screen is part of the flow.
  useEffect(() => {
    if (preview) {
      setAccessibility(isMacOS ? "needed" : "granted");
      setMicrophone(os === "other" ? "granted" : "needed");
      setNeeded(os !== "other");
      return;
    }
    if (os === "other") {
      setAccessibility("granted");
      setMicrophone("granted");
      return;
    }
    const generation = generationRef.current;
    const check = async () => {
      if (isMacOS) {
        try {
          const [acc, mic] = await readMac();
          if (!canApply(generation)) return;
          if (acc) await initializeAfterAccessibility();
          if (!canApply(generation)) return;
          setAccessibility(acc ? "granted" : "needed");
          setMicrophone(mic ? "granted" : "needed");
          setNeeded(!(acc && mic));
          if (acc && mic) await onAllGranted();
        } catch (error) {
          if (!canApply(generation)) return;
          console.error("Failed to check macOS permissions:", error);
          notify({ title: t("onboarding.permissions.errors.checkFailed") });
          setAccessibility("needed");
          setMicrophone("needed");
          setNeeded(true);
        }
        return;
      }
      setAccessibility("granted");
      try {
        const mic = await hasWindowsMicrophoneAccess();
        setMicrophone(mic ? "granted" : "needed");
        setNeeded(!mic);
        if (mic) await onAllGranted();
      } catch (error) {
        console.warn("Failed to check Windows microphone permissions:", error);
        setMicrophone("granted");
        await onAllGranted();
      }
    };
    check();
    // The first check runs once.
  }, []);

  const startPolling = useCallback(() => {
    if (
      pollingRef.current ||
      resetBusyRef.current ||
      resetEverSucceededRef.current
    )
      return;
    const generation = generationRef.current;
    pollingRef.current = setInterval(async () => {
      try {
        if (isWindows) {
          if (await hasWindowsMicrophoneAccess()) {
            setMicrophone("granted");
            stopPolling();
            await onAllGranted();
          }
          errorCountRef.current = 0;
          return;
        }
        const [acc, mic] = await readMac();
        if (!canApply(generation)) return;
        setAccessibility((prev) => {
          if (!canApply(generation)) return prev;
          if (acc && prev !== "granted") initializeAfterAccessibility();
          return acc ? "granted" : prev;
        });
        setMicrophone((prev) =>
          canApply(generation) && mic ? "granted" : prev,
        );
        if (acc && mic) {
          stopPolling();
          await onAllGranted();
        }
        errorCountRef.current = 0;
      } catch (error) {
        if (!canApply(generation)) return;
        console.error("Error checking permissions:", error);
        errorCountRef.current += 1;
        if (errorCountRef.current >= MAX_POLLING_ERRORS) {
          stopPolling();
          notify({ title: t("onboarding.permissions.errors.checkFailed") });
        }
      }
    }, 1000);
  }, [hasWindowsMicrophoneAccess, isWindows, onAllGranted, t]);

  useEffect(
    () => () => {
      generationRef.current += 1;
      stopPolling();
    },
    [],
  );

  // A waiting row whose answer was no (or never came): ask again.
  const recheckWaiting = useCallback(async () => {
    const generation = generationRef.current;
    if (!canApply(generation)) return;
    try {
      if (isWindows) {
        const mic = await hasWindowsMicrophoneAccess();
        setMicrophone((prev) =>
          prev === "waiting" ? (mic ? "granted" : "needed") : prev,
        );
        return;
      }
      if (!isMacOS) return;
      const [acc, mic] = await readMac();
      if (!canApply(generation)) return;
      setAccessibility((prev) =>
        canApply(generation) && prev === "waiting"
          ? acc
            ? "granted"
            : "needed"
          : prev,
      );
      setMicrophone((prev) =>
        prev === "waiting" ? (mic ? "granted" : "needed") : prev,
      );
    } catch (error) {
      console.warn("Failed to recheck permissions:", error);
    }
  }, [hasWindowsMicrophoneAccess, isMacOS, isWindows]);

  const waiting = microphone === "waiting" || accessibility === "waiting";
  useEffect(() => {
    if (preview || !waiting) return;
    const unlisten = getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) recheckWaiting();
    });
    // Only while Handy is in front: away in Settings, the row keeps waiting.
    const timer = setTimeout(() => {
      if (document.hasFocus()) recheckWaiting();
    }, WAITING_TIMEOUT_MS);
    return () => {
      clearTimeout(timer);
      unlisten.then((off) => off());
    };
  }, [preview, waiting, recheckWaiting]);

  const requestAccessibility = async () => {
    const generation = generationRef.current;
    if (preview || !canApply(generation)) return;
    try {
      await requestAccessibilityPermission();
      if (!canApply(generation)) return;
      setAccessibility("waiting");
      startPolling();
    } catch (error) {
      console.error("Failed to request accessibility permission:", error);
      notify({ title: t("onboarding.permissions.errors.requestFailed") });
    }
  };

  const requestMicrophone = async () => {
    const generation = generationRef.current;
    if (preview || !canApply(generation)) return;
    try {
      // macOS prompts once; after that, only System Settings can allow it.
      if (isWindows || microphoneAsked)
        await commands.openMicrophonePrivacySettings();
      else {
        await requestMicrophonePermission();
        // A prompt takes focus from Handy; still in front a moment later
        // means there was none (denied before), so go back to asking.
        setTimeout(() => {
          if (document.hasFocus()) recheckWaiting();
        }, 1000);
      }
      if (!canApply(generation)) return;
      setMicrophoneAsked(true);
      setMicrophone("waiting");
      startPolling();
    } catch (error) {
      console.error("Failed to request microphone permission:", error);
      notify({ title: t("onboarding.permissions.errors.requestFailed") });
    }
  };

  const resetAccessibility = async () => {
    if (preview || !isMacOS || resetBusyRef.current) return;
    // Invalidate reads before the OS reset starts, including the initial read
    // and focus/timeout rechecks. A successful reset stays blocked until restart.
    generationRef.current += 1;
    resetBusyRef.current = true;
    setResetBusy(true);
    stopPolling();
    setAccessibility("needed");
    setMicrophone((prev) => (prev === "waiting" ? "needed" : prev));
    try {
      const result = await commands.resetAccessibilityPermission();
      if (result.status === "error") throw new Error(result.error);
      resetEverSucceededRef.current = true;
      setRestartRequired(true);
      setResetDone(true);
    } catch (error) {
      console.error("Failed to reset accessibility permission:", error);
      setResetDone(false);
      notify({
        title: t("onboarding.permissions.troubleshoot.resetFailed", {
          command:
            "tccutil reset Accessibility ru.egorsokolov.klava-nevinovata",
        }),
        level: "error",
      });
    } finally {
      resetBusyRef.current = false;
      setResetBusy(false);
    }
  };

  return {
    platform: os,
    microphone,
    accessibility,
    checked: microphone !== "checking" && accessibility !== "checking",
    needed,
    allGranted,
    microphoneAsked,
    requestMicrophone,
    requestAccessibility,
    resetAccessibility,
    resetBusy,
    resetDone,
    restartRequired,
  };
};

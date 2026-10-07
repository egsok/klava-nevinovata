import { useEffect, useState, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  requestAccessibilityPermission,
  checkMicrophonePermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettingsStore } from "@/stores/settingsStore";
import HandyTextLogo from "../icons/HandyTextLogo";
import { Keyboard, Mic, Check, Loader2 } from "lucide-react";

interface AccessibilityOnboardingProps {
  onComplete: () => void;
  preview?: boolean;
}

type PermissionStatus = "checking" | "needed" | "waiting" | "granted";
type PermissionPlatform = "macos" | "windows" | "other";

interface PermissionsState {
  accessibility: PermissionStatus;
  microphone: PermissionStatus;
}

// Shown to the user when the automatic TCC reset fails; keep in sync with
// tauri.conf.json's identifier
const TCC_RESET_COMMAND =
  "tccutil reset Accessibility ru.egorsokolov.klava-nevinovata";

const AccessibilityOnboarding: React.FC<AccessibilityOnboardingProps> = ({
  onComplete,
  preview = false,
}) => {
  const { t } = useTranslation();
  const [showTroubleshoot, setShowTroubleshoot] = useState(false);
  // After a successful TCC reset the only reliable next step is a restart:
  // this process may keep seeing a stale "granted", so steer the user to the
  // Restart button instead of back to Grant Permission
  const [resetDone, setResetDone] = useState(false);
  // Monotonic companion to resetDone: once ANY reset succeeded in this
  // process, Grant must stay blocked even if a later re-reset fails (resetDone
  // then flips back to keep the panel texts honest). Ref is fine — every flip
  // of it is accompanied by a state update that re-renders.
  const resetEverSucceededRef = useRef(false);
  const refreshAudioDevices = useSettingsStore(
    (state) => state.refreshAudioDevices,
  );
  const refreshOutputDevices = useSettingsStore(
    (state) => state.refreshOutputDevices,
  );
  const [permissionPlatform, setPermissionPlatform] =
    useState<PermissionPlatform | null>(null);
  const [permissions, setPermissions] = useState<PermissionsState>({
    accessibility: "checking",
    microphone: "checking",
  });
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Bumped by the TCC reset so poll results that started before the reset
  // cannot promote state (and complete onboarding) with stale data
  const pollGenerationRef = useRef<number>(0);
  const initialCheckRanRef = useRef(false);
  const errorCountRef = useRef<number>(0);
  const MAX_POLLING_ERRORS = 3;
  // macOS TCC quirk: for unsigned builds the Accessibility toggle can sit "on"
  // while the OS still holds the grant for a previous binary — the poll then
  // never turns green. After a while, surface the remove-and-re-add recipe.
  const [showAccessibilityStuckHint, setShowAccessibilityStuckHint] =
    useState(false);

  useEffect(() => {
    if (permissions.accessibility !== "waiting") {
      setShowAccessibilityStuckHint(false);
      return;
    }
    const hintTimer = setTimeout(
      () => setShowAccessibilityStuckHint(true),
      8000,
    );
    return () => clearTimeout(hintTimer);
  }, [permissions.accessibility]);

  const isMacOS = permissionPlatform === "macos";
  const isWindows = permissionPlatform === "windows";
  const showMicrophonePermission = isMacOS || isWindows;
  const showAccessibilityPermission = isMacOS;

  const allGranted = isMacOS
    ? permissions.accessibility === "granted" &&
      permissions.microphone === "granted"
    : isWindows
      ? permissions.microphone === "granted"
      : true;

  const completeOnboarding = useCallback(async () => {
    const generation = pollGenerationRef.current;
    await Promise.all([refreshAudioDevices(), refreshOutputDevices()]);
    // A TCC reset may have landed while the device refresh was in flight;
    // don't arm the completion timeout on top of a reset
    if (generation !== pollGenerationRef.current) return;
    timeoutRef.current = setTimeout(() => onComplete(), 300);
  }, [onComplete, refreshAudioDevices, refreshOutputDevices]);

  const hasWindowsMicrophoneAccess = useCallback(async (): Promise<boolean> => {
    const microphoneStatus =
      await commands.getWindowsMicrophonePermissionStatus();

    if (!microphoneStatus.supported) {
      return true;
    }

    return microphoneStatus.overall_access !== "denied";
  }, []);

  // Check platform and permission status on mount
  useEffect(() => {
    // Strictly once: the deps (onComplete via App's render) change identity on
    // every parent re-render, and a re-run would overwrite permission state
    // with a fresh OS read — undoing a just-performed TCC reset
    if (initialCheckRanRef.current) return;
    initialCheckRanRef.current = true;

    const currentPlatform = platform();
    const nextPlatform: PermissionPlatform =
      currentPlatform === "macos"
        ? "macos"
        : currentPlatform === "windows"
          ? "windows"
          : "other";

    setPermissionPlatform(nextPlatform);

    // Debug previews are intentionally inert: show the permission request UI
    // without checking or changing operating-system permissions.
    if (preview) {
      setPermissions({
        accessibility: nextPlatform === "macos" ? "needed" : "granted",
        microphone: nextPlatform === "other" ? "granted" : "needed",
      });
      return;
    }

    // Skip immediately on unsupported platforms
    if (nextPlatform === "other") {
      onComplete();
      return;
    }

    const checkInitial = async () => {
      if (nextPlatform === "macos") {
        try {
          const [accessibilityGranted, microphoneGranted] = await Promise.all([
            checkAccessibilityPermission(),
            checkMicrophonePermission(),
          ]);

          // If accessibility is granted, initialize Enigo and shortcuts
          if (accessibilityGranted) {
            try {
              await Promise.all([
                commands.initializeEnigo(),
                commands.initializeShortcuts(),
              ]);
            } catch (e) {
              console.warn("Failed to initialize after permission grant:", e);
            }
          }

          const newState: PermissionsState = {
            accessibility: accessibilityGranted ? "granted" : "needed",
            microphone: microphoneGranted ? "granted" : "needed",
          };

          setPermissions(newState);

          if (accessibilityGranted && microphoneGranted) {
            await completeOnboarding();
          }
        } catch (error) {
          console.error("Failed to check macOS permissions:", error);
          toast.error(t("onboarding.permissions.errors.checkFailed"));
          setPermissions({
            accessibility: "needed",
            microphone: "needed",
          });
        }

        return;
      }

      try {
        const microphoneGranted = await hasWindowsMicrophoneAccess();

        setPermissions({
          accessibility: "granted",
          microphone: microphoneGranted ? "granted" : "needed",
        });

        if (microphoneGranted) {
          await completeOnboarding();
        }
      } catch (error) {
        console.warn("Failed to check Windows microphone permissions:", error);
        setPermissions({
          accessibility: "granted",
          microphone: "granted",
        });
        await completeOnboarding();
      }
    };

    checkInitial();
  }, [completeOnboarding, hasWindowsMicrophoneAccess, onComplete, preview, t]);

  // Polling for permissions after user clicks a button
  const startPolling = useCallback(() => {
    if (pollingRef.current || permissionPlatform === null) return;

    pollingRef.current = setInterval(async () => {
      const generation = pollGenerationRef.current;
      try {
        if (permissionPlatform === "windows") {
          const microphoneGranted = await hasWindowsMicrophoneAccess();
          if (generation !== pollGenerationRef.current) return;

          if (microphoneGranted) {
            setPermissions((prev) => ({ ...prev, microphone: "granted" }));

            if (pollingRef.current) {
              clearInterval(pollingRef.current);
              pollingRef.current = null;
            }

            await completeOnboarding();
          }

          errorCountRef.current = 0;
          return;
        }

        const [accessibilityGranted, microphoneGranted] = await Promise.all([
          checkAccessibilityPermission(),
          checkMicrophonePermission(),
        ]);
        if (generation !== pollGenerationRef.current) return;

        // After a successful TCC reset this process may keep reporting a
        // stale accessibility "granted" until the restart the flow asks for
        // — never promote it (or complete onboarding from it). Without this,
        // the microphone card's Grant button restarts polling and sneaks
        // onboarding past the disabled accessibility Grant.
        const accessibilityTrusted =
          accessibilityGranted && !resetEverSucceededRef.current;

        setPermissions((prev) => {
          const newState = { ...prev };

          if (accessibilityTrusted && prev.accessibility !== "granted") {
            newState.accessibility = "granted";
            // Initialize Enigo and shortcuts when accessibility is granted
            Promise.all([
              commands.initializeEnigo(),
              commands.initializeShortcuts(),
            ]).catch((e) => {
              console.warn("Failed to initialize after permission grant:", e);
            });
          }

          if (microphoneGranted && prev.microphone !== "granted") {
            newState.microphone = "granted";
          }

          return newState;
        });

        // If both granted, stop polling, refresh audio devices, and proceed
        if (accessibilityTrusted && microphoneGranted) {
          if (pollingRef.current) {
            clearInterval(pollingRef.current);
            pollingRef.current = null;
          }
          await completeOnboarding();
        }

        // Reset error count on success
        errorCountRef.current = 0;
      } catch (error) {
        console.error("Error checking permissions:", error);
        errorCountRef.current += 1;

        if (errorCountRef.current >= MAX_POLLING_ERRORS) {
          // Stop polling after too many consecutive errors
          if (pollingRef.current) {
            clearInterval(pollingRef.current);
            pollingRef.current = null;
          }
          toast.error(t("onboarding.permissions.errors.checkFailed"));
        }
      }
    }, 1000);
  }, [completeOnboarding, hasWindowsMicrophoneAccess, permissionPlatform, t]);

  // Cleanup polling and timeouts on unmount
  useEffect(() => {
    return () => {
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
      }
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  const handleGrantAccessibility = async () => {
    if (preview) return;

    try {
      await requestAccessibilityPermission();
      setPermissions((prev) => ({ ...prev, accessibility: "waiting" }));
      startPolling();
    } catch (error) {
      console.error("Failed to request accessibility permission:", error);
      toast.error(t("onboarding.permissions.errors.requestFailed"));
    }
  };

  const handleResetAccessibility = async () => {
    if (preview) return;

    try {
      const result = await commands.resetAccessibilityPermission();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      // Invalidate in-flight polls and any queued completion: a check that
      // started before the reset may still report the pre-reset "granted".
      // Stop polling too — the OS may keep reporting stale "granted" for a
      // while; the Grant button restarts polling when the user re-requests.
      pollGenerationRef.current += 1;
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
        pollingRef.current = null;
      }
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      // Demote local state: polling only ever promotes needed -> granted, so
      // without this the screen would keep showing a stale "granted". The
      // microphone card would otherwise spin forever on "waiting" — its
      // polling was just stopped together with the shared interval.
      setPermissions((prev) => ({
        ...prev,
        accessibility: "needed",
        microphone: prev.microphone === "waiting" ? "needed" : prev.microphone,
      }));
      resetEverSucceededRef.current = true;
      setResetDone(true);
      toast.success(t("onboarding.permissions.troubleshoot.resetSuccess"));
    } catch (error) {
      console.error("Failed to reset accessibility permission:", error);
      // A failed (re-)reset must not leave the panel claiming success
      setResetDone(false);
      toast.error(
        t("onboarding.permissions.troubleshoot.resetFailed", {
          command: TCC_RESET_COMMAND,
        }),
      );
    }
  };

  const handleRestartApp = async () => {
    if (preview) return;

    try {
      await relaunch();
    } catch (error) {
      console.error("Failed to relaunch app:", error);
      toast.error(t("onboarding.permissions.troubleshoot.restartFailed"));
    }
  };

  const handleGrantMicrophone = async () => {
    if (preview) return;

    try {
      if (isWindows) {
        await commands.openMicrophonePrivacySettings();
      } else {
        await requestMicrophonePermission();
      }

      setPermissions((prev) => ({ ...prev, microphone: "waiting" }));
      startPolling();
    } catch (error) {
      console.error("Failed to request microphone permission:", error);
      toast.error(t("onboarding.permissions.errors.requestFailed"));
    }
  };

  const isChecking =
    permissionPlatform === null ||
    (isMacOS &&
      permissions.accessibility === "checking" &&
      permissions.microphone === "checking") ||
    (isWindows && permissions.microphone === "checking");

  // Still checking platform/initial permissions
  if (isChecking) {
    return (
      <div className="h-screen w-full flex items-center justify-center">
        <Loader2 className="w-8 h-8 animate-spin text-text/50" />
      </div>
    );
  }

  // All permissions granted - show success briefly
  if (allGranted) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center gap-4">
        <div className="p-4 rounded-full bg-state-soft/20">
          <Check className="w-12 h-12 text-state-soft" />
        </div>
        <p className="text-lg font-medium text-text">
          {t("onboarding.permissions.allGranted")}
        </p>
      </div>
    );
  }

  // Show permissions request screen
  return (
    <div className="h-screen w-full flex flex-col p-6 gap-6 items-center justify-center">
      <div className="flex flex-col items-center gap-2">
        <HandyTextLogo width={200} />
      </div>

      <div className="max-w-md w-full flex flex-col items-center gap-4">
        <div className="text-center mb-2">
          <h2 className="text-xl font-semibold text-text mb-2">
            {t("onboarding.permissions.title")}
          </h2>
          <p className="text-text/70">
            {t("onboarding.permissions.description")}
          </p>
        </div>

        {/* Microphone Permission Card */}
        {showMicrophonePermission && (
          <div className="w-full p-4 rounded-lg bg-white/5 border border-mid-gray/20">
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-full bg-logo-primary/20 shrink-0">
                <Mic className="w-6 h-6 text-logo-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-text">
                  {t("onboarding.permissions.microphone.title")}
                </h3>
                <p className="text-sm text-text/60 mb-3">
                  {t("onboarding.permissions.microphone.description")}
                </p>
                {permissions.microphone === "granted" ? (
                  <div className="flex items-center gap-2 text-state-soft text-sm">
                    <Check className="w-4 h-4" />
                    {t("onboarding.permissions.granted")}
                  </div>
                ) : permissions.microphone === "waiting" ? (
                  <div className="flex items-center gap-2 text-text/50 text-sm">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t("onboarding.permissions.waiting")}
                  </div>
                ) : (
                  <button
                    onClick={handleGrantMicrophone}
                    className="px-4 py-2 rounded-lg bg-logo-primary hover:bg-logo-primary/90 text-white text-sm font-medium transition-colors"
                  >
                    {isWindows
                      ? t("accessibility.openSettings")
                      : t("onboarding.permissions.grant")}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Accessibility Permission Card */}
        {showAccessibilityPermission && (
          <div className="w-full p-4 rounded-lg bg-white/5 border border-mid-gray/20">
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-full bg-logo-primary/20 shrink-0">
                <Keyboard className="w-6 h-6 text-logo-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-text">
                  {t("onboarding.permissions.accessibility.title")}
                </h3>
                <p className="text-sm text-text/60 mb-3">
                  {t("onboarding.permissions.accessibility.description")}
                </p>
                {permissions.accessibility === "granted" ? (
                  <div className="flex items-center gap-2 text-state-soft text-sm">
                    <Check className="w-4 h-4" />
                    {t("onboarding.permissions.granted")}
                  </div>
                ) : permissions.accessibility === "waiting" ? (
                  <div className="flex flex-col gap-2">
                    <div className="flex items-center gap-2 text-text/50 text-sm">
                      <Loader2 className="w-4 h-4 animate-spin" />
                      {t("onboarding.permissions.waiting")}
                    </div>
                    {showAccessibilityStuckHint && (
                      <>
                        <p className="text-xs text-text/60">
                          {t("onboarding.permissions.accessibility.stuckHint")}
                        </p>
                        <button
                          onClick={handleGrantAccessibility}
                          className="self-start text-xs text-logo-primary underline underline-offset-2 hover:opacity-80"
                        >
                          {t(
                            "onboarding.permissions.accessibility.openSystemSettings",
                          )}
                        </button>
                      </>
                    )}
                  </div>
                ) : (
                  <button
                    onClick={handleGrantAccessibility}
                    // After a TCC reset this process may still see a stale
                    // "granted" and would complete onboarding without a real
                    // permission — the only reliable next step is a restart
                    disabled={resetEverSucceededRef.current}
                    className="px-4 py-2 rounded-lg bg-logo-primary hover:bg-logo-primary/90 text-white text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    {t("onboarding.permissions.grant")}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Troubleshooting for stale TCC entries (old Handy / old builds) */}
        {showAccessibilityPermission &&
          permissions.accessibility !== "granted" && (
            <div className="w-full flex flex-col items-center">
              <button
                onClick={() => setShowTroubleshoot((prev) => !prev)}
                className="text-sm text-text/50 hover:text-text/80 underline transition-colors cursor-pointer"
              >
                {t("onboarding.permissions.troubleshoot.link")}
              </button>
              {/* Once a reset succeeded, Grant is blocked for good — keep the
                  panel (with the Restart button) open so the blocked button
                  never sits there unexplained */}
              {(showTroubleshoot || resetEverSucceededRef.current) && (
                <div className="w-full mt-2 p-4 rounded-lg bg-white/5 border border-mid-gray/20 flex flex-col gap-3">
                  {resetDone && (
                    <p className="text-sm font-medium text-text">
                      {t("onboarding.permissions.troubleshoot.resetSuccess")}
                    </p>
                  )}
                  {/* Keep the remove-stale-entries recipe visible even after a
                      reset — it may be the user's actual problem */}
                  <p className="text-sm text-text/60">
                    {t("onboarding.permissions.troubleshoot.description")}
                  </p>
                  <div className="flex flex-wrap gap-2">
                    <button
                      onClick={handleResetAccessibility}
                      className="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-text text-sm font-medium transition-colors"
                    >
                      {t("onboarding.permissions.troubleshoot.resetButton")}
                    </button>
                    <button
                      onClick={handleRestartApp}
                      className={
                        resetDone
                          ? "px-3 py-1.5 rounded-lg bg-logo-primary hover:bg-logo-primary/90 text-white text-sm font-medium transition-colors"
                          : "px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-text text-sm font-medium transition-colors"
                      }
                    >
                      {t("onboarding.permissions.troubleshoot.restartButton")}
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
      </div>
    </div>
  );
};

export default AccessibilityOnboarding;

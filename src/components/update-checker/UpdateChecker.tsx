import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { arch, platform } from "@tauri-apps/plugin-os";
import { ProgressBar } from "../shared";
import { useSettings } from "../../hooks/useSettings";
import { commands } from "../../bindings";
import {
  resolvePortableInstallerUrl,
  PORTABLE_RELEASES_URL,
} from "./portableInstaller";

interface UpdateCheckerProps {
  className?: string;
}

const UpdateChecker: React.FC<UpdateCheckerProps> = ({ className = "" }) => {
  const { t } = useTranslation();
  // Update checking state
  const [isChecking, setIsChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [showUpToDate, setShowUpToDate] = useState(false);
  const [showPortableUpdateDialog, setShowPortableUpdateDialog] =
    useState(false);
  const [portableInstallerUrl, setPortableInstallerUrl] = useState<string>(
    PORTABLE_RELEASES_URL,
  );

  const { settings, isLoading, updateChecksLocked } = useSettings();
  // Wait for the lock state too (null = not loaded yet), otherwise the first
  // render could fire an update check before HANDY_DISABLE_UPDATER is known.
  const settingsLoaded =
    !isLoading && settings !== null && updateChecksLocked !== null;
  // Forced-off by system configuration (HANDY_DISABLE_UPDATER) overrides the
  // stored preference without persisting it, mirroring the backend's effective
  // updater state.
  const updateChecksEnabled =
    (settings?.update_checks_enabled ?? true) && updateChecksLocked === false;

  const upToDateTimeoutRef = useRef<ReturnType<typeof setTimeout>>();
  const isManualCheckRef = useRef(false);
  // Refs, not state: the tray-event listener captures the mount-time render,
  // so a state flag would never guard against a concurrent check/install
  const isCheckingRef = useRef(false);
  const isInstallingRef = useRef(false);
  const downloadedBytesRef = useRef(0);
  const contentLengthRef = useRef(0);

  useEffect(() => {
    // Wait for settings to load before doing anything
    if (!settingsLoaded) return;

    if (!updateChecksEnabled) {
      if (upToDateTimeoutRef.current) {
        clearTimeout(upToDateTimeoutRef.current);
      }
      setIsChecking(false);
      setUpdateAvailable(false);
      setShowUpToDate(false);
      return;
    }

    checkForUpdates();

    // Listen for update check events
    const updateUnlisten = listen("check-for-updates", () => {
      handleManualUpdateCheck();
    });

    return () => {
      if (upToDateTimeoutRef.current) {
        clearTimeout(upToDateTimeoutRef.current);
      }
      updateUnlisten.then((fn) => fn());
    };
  }, [settingsLoaded, updateChecksEnabled]);

  // Update checking functions
  const checkForUpdates = async () => {
    if (isCheckingRef.current || isInstallingRef.current) return;

    try {
      isCheckingRef.current = true;
      setIsChecking(true);
      const update = await check();

      if (update) {
        setUpdateAvailable(true);
        setShowUpToDate(false);
        // Portable installs can't self-update in place — the manual dialog links
        // straight at the matching installer from this manifest instead.
        setPortableInstallerUrl(
          resolvePortableInstallerUrl(update.rawJson, platform(), arch()),
        );
      } else {
        setUpdateAvailable(false);

        if (isManualCheckRef.current) {
          setShowUpToDate(true);
          if (upToDateTimeoutRef.current) {
            clearTimeout(upToDateTimeoutRef.current);
          }
          upToDateTimeoutRef.current = setTimeout(() => {
            setShowUpToDate(false);
          }, 3000);
        }
      }
    } catch (error) {
      console.error("Failed to check for updates:", error);
      // Only a user-initiated check gets a toast; the automatic one on every
      // launch must stay quiet offline
      if (isManualCheckRef.current) {
        toast.error(t("footer.updateCheckFailed"));
      }
    } finally {
      isCheckingRef.current = false;
      setIsChecking(false);
      isManualCheckRef.current = false;
    }
  };

  const handleManualUpdateCheck = () => {
    isManualCheckRef.current = true;
    checkForUpdates();
  };

  const installUpdate = async () => {
    const portable = await commands.isPortable();
    if (portable) {
      setShowPortableUpdateDialog(true);
      return;
    }

    try {
      isInstallingRef.current = true;
      setIsInstalling(true);
      setDownloadProgress(0);
      downloadedBytesRef.current = 0;
      contentLengthRef.current = 0;
      const update = await check();

      if (!update) {
        // The update disappeared between the check and the click (e.g. release
        // pulled); reflect reality instead of silently ignoring the click
        console.log("No update available during install attempt");
        setUpdateAvailable(false);
        return;
      }

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            downloadedBytesRef.current = 0;
            contentLengthRef.current = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloadedBytesRef.current += event.data.chunkLength;
            const progress =
              contentLengthRef.current > 0
                ? Math.round(
                    (downloadedBytesRef.current / contentLengthRef.current) *
                      100,
                  )
                : 0;
            setDownloadProgress(Math.min(progress, 100));
            break;
        }
      });
      await relaunch();
    } catch (error) {
      console.error("Failed to install update:", error);
      toast.error(t("footer.updateInstallFailed"));
    } finally {
      isInstallingRef.current = false;
      setIsInstalling(false);
      setDownloadProgress(0);
      downloadedBytesRef.current = 0;
      contentLengthRef.current = 0;
    }
  };

  // Update status functions
  const getUpdateStatusText = () => {
    if (isInstalling) {
      return downloadProgress > 0 && downloadProgress < 100
        ? t("footer.downloading", {
            progress: downloadProgress.toString().padStart(3),
          })
        : downloadProgress === 100
          ? t("footer.installing")
          : t("footer.preparing");
    }
    if (isChecking) return t("footer.checkingUpdates");
    if (showUpToDate) return t("footer.upToDate");
    if (updateAvailable) return t("footer.updateAvailableShort");
    return t("footer.checkForUpdates");
  };

  const getUpdateStatusAction = () => {
    if (updateAvailable && !isInstalling) return installUpdate;
    if (!isChecking && !isInstalling && !updateAvailable)
      return handleManualUpdateCheck;
    return undefined;
  };

  const isUpdateDisabled = isChecking || isInstalling;
  const isUpdateClickable =
    !isUpdateDisabled && (updateAvailable || (!isChecking && !showUpToDate));

  // When no installer could be resolved for this target the button falls back to
  // the releases index, so the dialog has to say "browse" rather than "download".
  const hasDirectInstaller = portableInstallerUrl !== PORTABLE_RELEASES_URL;

  // Nothing until settings arrive: rendering on the `?? true` fallback would
  // flash the button for users who turned update checks off. Turned off means
  // quiet: no permanent "checking disabled" label in the footer chrome (the
  // tray item is greyed out by the same setting)
  if (!settingsLoaded || !updateChecksEnabled) return null;

  return (
    <>
      {showPortableUpdateDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border border-mid-gray/20 rounded-lg p-6 max-w-md w-full mx-4 space-y-4">
            <h2 className="text-base font-semibold">
              {t("footer.portableUpdateTitle")}
            </h2>
            <p className="text-sm text-text/70">
              {hasDirectInstaller
                ? t("footer.portableUpdateMessage")
                : t("footer.portableUpdateBrowseMessage")}
            </p>
            <div className="flex gap-2 justify-end">
              <button
                className="px-3 py-1.5 text-sm rounded border border-mid-gray/20 hover:bg-mid-gray/10 transition-colors"
                onClick={() => setShowPortableUpdateDialog(false)}
              >
                {t("common.close")}
              </button>
              <button
                className="px-3 py-1.5 text-sm rounded bg-logo-primary text-white hover:bg-logo-primary/80 transition-colors"
                onClick={() => {
                  openUrl(portableInstallerUrl);
                  setShowPortableUpdateDialog(false);
                }}
              >
                {hasDirectInstaller
                  ? t("footer.portableUpdateButton")
                  : t("footer.portableUpdateBrowseButton")}
              </button>
            </div>
          </div>
        </div>
      )}
      <div className={`flex items-center gap-3 min-w-0 ${className}`}>
        {isUpdateClickable ? (
          <button
            onClick={getUpdateStatusAction()}
            disabled={isUpdateDisabled}
            className={`transition-colors disabled:opacity-50 tabular-nums min-w-0 truncate ${
              updateAvailable
                ? "text-logo-primary hover:text-logo-primary/80 font-medium"
                : "text-text/60 hover:text-text/80"
            }`}
          >
            {getUpdateStatusText()}
          </button>
        ) : (
          <span className="text-text/60 tabular-nums min-w-0 truncate">
            {getUpdateStatusText()}
          </span>
        )}

        {isInstalling && downloadProgress > 0 && downloadProgress < 100 && (
          <ProgressBar
            progress={[
              {
                id: "update",
                percentage: downloadProgress,
              },
            ]}
            size="large"
            className="shrink-0"
          />
        )}
        {/* Separator lives here so it disappears together with the checker;
            -ml-2 trims this flexbox's gap-3 down to the footer's gap-1 rhythm */}
        <span aria-hidden="true" className="-ml-2">
          •
        </span>
      </div>
    </>
  );
};

export default UpdateChecker;

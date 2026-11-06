import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import AppMark from "../../icons/AppMark";
import HandyTextLogo from "../../icons/HandyTextLogo";
import { AppDataDirectory } from "../AppDataDirectory";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { ShowWhatsNewOnUpdate } from "../ShowWhatsNewOnUpdate";
import { ThemeSelector } from "../ThemeSelector";
import { LogDirectory } from "../debug";
import { REPO_URL, TELEGRAM_URL_ABOUT } from "@/lib/constants/brand";

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="flex items-center gap-4 border border-mid-gray/20 rounded-lg p-4">
        <AppMark width={52} height={52} className="shrink-0" />
        <div className="min-w-0">
          <HandyTextLogo width={180} />
          <div className="text-xs text-mid-gray mt-1">
            {/* eslint-disable-next-line i18next/no-literal-string */}
            <span className="font-mono">v{version} · </span>
            {t("settings.about.brand.tagline")}
          </div>
        </div>
      </div>

      <SettingsGroup title={t("settings.about.title")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <ThemeSelector descriptionMode="tooltip" grouped={true} />
        <ShowWhatsNewOnUpdate descriptionMode="tooltip" grouped={true} />

        <SettingContainer
          title={t("settings.about.sourceCode.title")}
          description={t("settings.about.sourceCode.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl(REPO_URL)}
          >
            {t("settings.about.sourceCode.button")}
          </Button>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.telegram.title")}
          description={t("settings.about.telegram.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl(TELEGRAM_URL_ABOUT)}
          >
            {t("settings.about.telegram.button")}
          </Button>
        </SettingContainer>

        <AppDataDirectory descriptionMode="tooltip" grouped={true} />
        <LogDirectory grouped={true} />
      </SettingsGroup>

      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <SettingContainer
          title={t("settings.about.acknowledgments.ggml.title")}
          description={t("settings.about.acknowledgments.ggml.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-ink-3">
            {t("settings.about.acknowledgments.ggml.details")}
          </div>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};

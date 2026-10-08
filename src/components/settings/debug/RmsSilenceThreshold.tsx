import React from "react";
import { useTranslation } from "react-i18next";
import { Slider } from "../../ui/Slider";
import { useSettings } from "../../../hooks/useSettings";

interface RmsSilenceThresholdProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const RmsSilenceThreshold: React.FC<RmsSilenceThresholdProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { settings, updateSetting, resetSetting, isUpdating } = useSettings();

  const handleThresholdChange = (value: number) => {
    updateSetting("rms_silence_threshold", value);
  };

  return (
    <Slider
      value={settings?.rms_silence_threshold ?? 0.005}
      onChange={handleThresholdChange}
      onReset={() => resetSetting("rms_silence_threshold")}
      isResetting={isUpdating("rms_silence_threshold")}
      min={0.0005}
      max={0.02}
      step={0.0005}
      formatValue={(v) => v.toFixed(4)}
      label={t("settings.debug.rmsSilenceThreshold.title")}
      description={t("settings.debug.rmsSilenceThreshold.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    />
  );
};

import React from "react";
import { Trans } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { TextLink } from "../ui/TextLink";

const LINUX_SETUP_URL =
  "https://github.com/egsok/klava-nevinovata/blob/main/docs/troubleshooting/ubuntu-26-04-gnome-wayland/README.md";

/** Linux footer hint linking the setup guide; shortcuts and pasting vary by desktop. */
export const LinuxGuide: React.FC<{
  i18nKey: "onboarding.linux.tryIt" | "onboarding.linux.done";
}> = ({ i18nKey }) => (
  <Trans i18nKey={i18nKey} components={{ guide: <GuideLink /> }} />
);

const GuideLink: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <TextLink
    className="inline-flex items-center gap-[3px]"
    onClick={() => openUrl(LINUX_SETUP_URL).catch(console.error)}
  >
    {children}
    <ExternalLink className="size-[11px]" aria-hidden="true" />
  </TextLink>
);

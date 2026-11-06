import React, { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";

import ModelSelector from "../model-selector";
import UpdateChecker from "../update-checker";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Send } from "lucide-react";
import {
  TELEGRAM_CHANNEL_NAME,
  TELEGRAM_URL_FOOTER,
} from "@/lib/constants/brand";
import NoticeChip from "./NoticeChip";
import TranscriptionStatus from "./TranscriptionStatus";

interface FooterProps {
  onShowHistory: () => void;
}

const Footer: React.FC<FooterProps> = ({ onShowHistory }) => {
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
    <div className="w-full border-t border-line pt-3">
      <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] gap-x-4 items-center font-mono text-[12px] font-medium px-4 pb-3 text-ink-2">
        <div className="flex items-center gap-3 min-w-0 justify-self-start">
          <ModelSelector />
          <TranscriptionStatus onShowHistory={onShowHistory} />
        </div>

        <button
          onClick={() => openUrl(TELEGRAM_URL_FOOTER)}
          className="flex items-center gap-1.5 justify-self-center hover:text-logo-primary transition-colors cursor-pointer"
        >
          <Send size={12} className="shrink-0" />
          <span>{TELEGRAM_CHANNEL_NAME}</span>
        </button>
        <div className="flex items-center gap-3 min-w-0 justify-self-end">
          {/* Update Status */}
          <div className="flex items-center gap-1 min-w-0">
            <UpdateChecker />
            {/* eslint-disable-next-line i18next/no-literal-string */}
            <span className="shrink-0">v{version}</span>
          </div>
          <NoticeChip />
        </div>
      </div>
    </div>
  );
};

export default Footer;

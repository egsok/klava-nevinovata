import type { TFunction } from "i18next";
import { ASR_BY_REPO } from "@/lib/models/catalog";
import type { ModelInfo } from "@/bindings";
import { repoIdOf } from "@/lib/models/retired";
import {
  getTranslatedModelDescription,
  getTranslatedModelName,
} from "@/lib/utils/modelTranslation";
import type { TableModel } from "./context";

// Legacy (Url-sourced .bin) models still run from disk but aren't offered.
export const isLegacyModel = (model: ModelInfo): boolean =>
  typeof model.source === "object" && "Url" in model.source;

// Extract a GGUF quantization label from a filename, if present (e.g. "Q8_0").
const quantOf = (filename: string): string | null => {
  const match = filename.match(
    /[._-](IQ\d+_\w+|Q\d+(?:_\w+)?|F16|BF16|F32)\.gguf$/i,
  );
  return match ? match[1].toUpperCase() : null;
};

/** An app model as the table shows it, joined to its catalog entry by repo. */
export const toTableModel = (info: ModelInfo, t: TFunction): TableModel => {
  const repo = repoIdOf(info);
  const model = info.is_custom || !repo ? undefined : ASR_BY_REPO.get(repo);
  return {
    id: info.id,
    name: getTranslatedModelName(info, t),
    description: getTranslatedModelDescription(info, t),
    languages: info.supported_languages,
    streaming: info.supports_streaming,
    translate: info.supports_translation,
    sizeMb: Number(info.size_mb),
    // The file's own, so a different quant found on disk says so.
    quant: quantOf(info.filename) ?? model?.quant ?? null,
    kind: info.is_custom
      ? "custom"
      : isLegacyModel(info)
        ? "legacy"
        : model?.curated
          ? "curated"
          : "catalog",
    model:
      model ??
      (!info.is_custom &&
      isLegacyModel(info) &&
      (info.id === "breeze-asr" || info.id === "turbo")
        ? {
            id: info.id,
            languages: info.supported_languages,
            family: "whisper",
          }
        : undefined),
  };
};

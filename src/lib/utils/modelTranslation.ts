import type { TFunction } from "i18next";
import type { ModelInfo } from "@/bindings";
import { ASR_BY_REPO, BREEZE, TURBO } from "@/lib/models/catalog";
import { repoIdOf } from "@/lib/models/retired";

export function dictationModelDescription(id: string, t: TFunction) {
  if (id === BREEZE || id === "breeze-asr")
    return t("onboarding.dictationModels.breeze");
  if (id === TURBO || id === "turbo")
    return t("onboarding.dictationModels.turbo");
}

/**
 * Get the translated name for a model
 * @param model - The model info object
 * @param t - The translation function from useTranslation
 * @returns The translated model name, or the original name if no translation exists
 */
export function getTranslatedModelName(model: ModelInfo, t: TFunction): string {
  const translationKey = `onboarding.models.${model.id}.name`;
  const translated = t(translationKey, { defaultValue: "" });
  return translated !== "" ? translated : model.name;
}

/**
 * Get the translated description for a model
 * @param model - The model info object
 * @param t - The translation function from useTranslation
 * @returns The translated model description, or the original description if no translation exists
 */
export function getTranslatedModelDescription(
  model: ModelInfo,
  t: TFunction,
): string {
  // Custom models use a generic translation key
  if (model.is_custom) {
    return t("onboarding.customModelDescription");
  }
  const repo = repoIdOf(model);
  const catalogId = (repo && ASR_BY_REPO.get(repo)?.id) || model.id;
  const description = dictationModelDescription(catalogId, t);
  if (description) return description;
  const translationKey = `onboarding.models.${model.id}.description`;
  const translated = t(translationKey, { defaultValue: "" });
  return translated !== "" ? translated : model.description;
}

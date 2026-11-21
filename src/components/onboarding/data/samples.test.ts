import assert from "node:assert/strict";
import en from "@/i18n/locales/en/translation.json";
import { demoLine, SAMPLES } from "./samples";

const translate = (key: string, { lng }: { lng: string }) => {
  assert.equal(lng, "en");
  return key === "onboarding.demo.chat"
    ? en.onboarding.demo.chat
    : en.onboarding.demo.noteTitle;
};

// Dictation languages never enter this function: both chapters use the UI language.
const chat = demoLine("ru", "zh-Hans", "hold", translate);
const note = demoLine("ru-RU", "zh-Hans", "tap", translate);
assert.equal(chat.language, "ru");
assert.match(chat.context, /макеты/);
assert.match(chat.text, /три часа/);
assert.equal(note.language, "ru");
assert.match(note.context, /дизайн-ревью/);
assert.match(note.text, /макеты/);

for (const mode of ["hold", "tap"] as const) {
  // A language without a prepared example gets the entire English scene.
  assert.deepEqual(
    demoLine("sv", "zh-Hans", mode, translate),
    demoLine("en", "zh-Hans", mode, translate),
  );
  for (const lang of Object.keys(SAMPLES)) {
    const script = lang === "zh-HK" || lang === "zh-Hant" ? lang : "zh-Hans";
    const example = demoLine(
      lang.startsWith("zh-") ? "zh-TW" : lang,
      script,
      mode,
      translate,
    );
    assert.equal(example.language, lang);
    assert.ok(example.context && example.text);
  }
}

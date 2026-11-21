/** Demo's dictated lines per language. Sample speech, not UI copy, so not in i18n. */
import {
  isChinese,
  type ChineseScriptChoice,
} from "@/lib/utils/languageSuggestions";

export const SAMPLES: Record<string, string> = {
  es: "Oye, ¿podemos mover la revisión de diseño de mañana a las tres? Quiero un poco más de tiempo para terminar las maquetas.",
  fr: "Est-ce qu'on peut décaler la revue de design de demain à quinze heures ? J'aimerais un peu plus de temps pour finir les maquettes.",
  de: "Können wir die Design-Besprechung morgen auf drei Uhr verschieben? Ich brauche noch etwas Zeit für die Entwürfe.",
  ja: "明日のデザインレビューを三時に変更できますか？モックアップを仕上げる時間が少し欲しいです。",
  ko: "내일 디자인 리뷰를 세 시로 옮길 수 있을까요? 시안을 마무리할 시간이 조금 더 필요해요.",
  "zh-Hans": "我们明天的设计评审能改到三点吗？我想多一点时间把原型做完。",
  "zh-Hant": "我們明天的設計評審能改到三點嗎？我想多一點時間把原型做完。",
  "zh-HK": "我哋聽日嘅設計評審可唔可以改到三點？我想多啲時間做完個原型。",
  pt: "Podemos mudar a revisão de design de amanhã para as três? Quero um pouco mais de tempo para terminar os protótipos.",
  ru: "Можем перенести завтрашнее дизайн-ревью на три часа? Нужно доделать макеты к завтрашнему обсуждению.",
  it: "Possiamo spostare la revisione del design di domani alle tre? Vorrei un po' più di tempo per finire i prototipi.",
};

// Each chat opener and note title belongs to the example, not the app's chrome.
const CONTEXT: Record<string, { hold: string; tap: string }> = {
  es: {
    hold: "¿A qué hora revisamos los diseños mañana?",
    tap: "Preparar la revisión de diseño",
  },
  fr: {
    hold: "À quelle heure regarde-t-on les maquettes demain ?",
    tap: "Préparer la revue de design",
  },
  de: {
    hold: "Wann besprechen wir morgen die Entwürfe?",
    tap: "Vorbereitung auf die Designbesprechung",
  },
  ja: {
    hold: "明日のデザインレビューは何時にしましょうか？",
    tap: "デザインレビューの準備",
  },
  ko: { hold: "내일 디자인 리뷰는 몇 시에 할까요?", tap: "디자인 리뷰 준비" },
  "zh-Hans": { hold: "明天几点看设计稿？", tap: "设计评审准备" },
  "zh-Hant": { hold: "明天幾點看設計稿？", tap: "設計評審準備" },
  "zh-HK": { hold: "聽日幾點睇設計稿？", tap: "設計評審準備" },
  pt: {
    hold: "A que horas vamos rever os protótipos amanhã?",
    tap: "Preparação para a revisão de design",
  },
  ru: { hold: "Во сколько завтра обсудим макеты?", tap: "К дизайн-ревью" },
  it: {
    hold: "A che ora rivediamo i prototipi domani?",
    tap: "Preparazione della revisione del design",
  },
};

const ENGLISH = {
  hold: "Yes! Running ten minutes late, save me a seat.",
  tap: "Picnic in the park on Saturday, then dinner at Rosa's. Bring the blue blanket and the good speaker.",
};

/** Both scenes follow the interface language. Dictation preferences concern real speech. */
export const demoLine = (
  uiLanguage: string,
  script: ChineseScriptChoice,
  mode: "hold" | "tap",
  translate: (key: string, options: { lng: string }) => string,
): { language: string; context: string; text: string } => {
  const i = mode === "hold" ? 0 : 1;
  const ui = uiLanguage.toLowerCase().split("-")[0];
  const language = isChinese(ui) ? script : ui;
  const sample = SAMPLES[language];
  if (!sample)
    return {
      language: "en",
      context: translate(
        mode === "hold" ? "onboarding.demo.chat" : "onboarding.demo.noteTitle",
        { lng: "en" },
      ),
      text: ENGLISH[mode],
    };
  const sentences = sample.split(/(?<=[?？。！!.])\s*/).filter(Boolean);
  return {
    language,
    context: CONTEXT[language][mode],
    text: sentences[i % sentences.length],
  };
};

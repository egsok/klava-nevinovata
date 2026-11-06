// klava-nevinovata brand asset generator — v5 (2026-07-16)
// v5: two-ink repaint ("Нейросеть не виновата"): violet capsule/outline,
//     magenta halo printed +24 units off registration (variant 2 — the inks
//     didn't converge, не виновата). Geometry untouched, so the mono tray
//     silhouettes are unchanged. Wordmark moves to IBM Plex Mono, caret
//     goes magenta.
// v4: halo got an ink outline (matches capsule), cradle arc hugs the capsule
//     (uniform 55-unit gap; the old deep bowl wasted ~15% of canvas height),
//     mark enlarged +20%, composition vertically balanced (was 9px from top).
//     Tray glyph iter2 (per Windows mic reference): cradle hugs the capsule
//     (~1px gap at 16px), narrower capsule, compact halo kept fully inside
//     the canvas (the tilt widens the ellipse bbox — iter1 clipped 1.5 units).
// v3: keycap dropped — mark is now "mic + halo" (keycap didn't survive 16-24px;
//     Egor's call). Klava meaning is carried by the wordmark.
// v2: increased glyph scale.
// Writes master SVGs and renders PNGs into D:/dev/Handy-v09/rebrand/
import { Resvg } from "@resvg/resvg-js";
import { mkdirSync, writeFileSync } from "fs";
import { join } from "path";

const OUT = "D:/dev/Handy-v09/rebrand";
for (const d of ["", "svg", "tray", "wordmark"])
  mkdirSync(join(OUT, d), { recursive: true });

// ---- palette: the two inks + paper ----
const VIOLET = "#2c1a72"; // second ink — capsule, cradle, halo outline
const MAGENTA = "#e11b76"; // first ink — halo, recording, action
const PLATE = "#f2ecdc"; // paper-2, the printed sheet
const CREAM = "#f2ecdc"; // type/strokes over solid ink (same sheet tone)
const MONO_WHITE = "#FFFFFF"; // tray icons on dark taskbar (no *_dark suffix)
const MONO_INK = "#1E1913"; // tray icons on light taskbar (*_dark suffix)

const svgOpen = (vb) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}">`;

// ---- app logo 1024: big mic + halo on warm plate ----
// Halo is two concentric strokes (ink under amber) — SVG strokes can't have
// their own outline. Halo is drawn first so its lower edge tucks behind the
// capsule.
const logo = `${svgOpen("0 0 1024 1024")}
  <rect x="32" y="32" width="960" height="960" rx="212" fill="${PLATE}"/>
  <ellipse cx="524" cy="238" rx="258" ry="80" fill="none" stroke="${VIOLET}" stroke-width="94" transform="rotate(-8 524 238)"/>
  <ellipse cx="548" cy="262" rx="258" ry="80" fill="none" stroke="${MAGENTA}" stroke-width="62" transform="rotate(-8 548 262)"/>
  <rect x="332" y="347" width="360" height="420" rx="180" fill="${VIOLET}"/>
  <path d="M 238 587 a 274 274 0 0 0 548 0" fill="none" stroke="${VIOLET}" stroke-width="48" stroke-linecap="round"/>
  <line x1="512" y1="861" x2="512" y2="895" stroke="${VIOLET}" stroke-width="48" stroke-linecap="round"/>
</svg>`;

// ---- tray icons 64: mic + halo, glyph fills the canvas ----
// idle = outline capsule, recording = filled capsule, transcribing = three dots.
const HALO = (c) =>
  `<ellipse cx="32.5" cy="9.2" rx="16.5" ry="4.8" fill="none" stroke="${c}" stroke-width="5.5" transform="rotate(-9 32.5 9.2)"/>`;
const ARC = (c, sw = 5.5) =>
  `<path d="M 12.5 37.5 a 19.5 19.5 0 0 0 39 0" fill="none" stroke="${c}" stroke-width="${sw}" stroke-linecap="round"/>
   <line x1="32" y1="57" x2="32" y2="59.5" stroke="${c}" stroke-width="${sw}" stroke-linecap="round"/>`;
const CAPSULE = (attrs) =>
  `<rect x="21.5" y="18" width="21" height="30" rx="10.5" ${attrs}/>`;
const DOTS = (c) =>
  `<circle cx="19" cy="33" r="4.8" fill="${c}"/><circle cx="32" cy="33" r="4.8" fill="${c}"/><circle cx="45" cy="33" r="4.8" fill="${c}"/>`;

function trayMono(c, state) {
  let body = "";
  if (state === "idle")
    body = CAPSULE(`fill="none" stroke="${c}" stroke-width="5.5"`) + ARC(c);
  if (state === "recording") body = CAPSULE(`fill="${c}"`) + ARC(c);
  if (state === "transcribing") body = DOTS(c) + ARC(c);
  return `${svgOpen("0 0 64 64")}${HALO(c)}${body}</svg>`;
}

// Colored theme: arc/stem paper-cream (colored tray lives on dark taskbars;
// violet would vanish there). Recording = magenta: the ink of action.
function trayColored(state) {
  let body = "";
  if (state === "idle") body = CAPSULE(`fill="${VIOLET}"`) + ARC(CREAM);
  if (state === "recording") body = CAPSULE(`fill="${MAGENTA}"`) + ARC(CREAM);
  if (state === "transcribing") body = DOTS(CREAM) + ARC(CREAM);
  return `${svgOpen("0 0 64 64")}${HALO(MAGENTA)}${body}</svg>`;
}

// ---- wordmark ----
// textLength pins glyph advances (same values as HandyTextLogo.tsx) so the
// text ends at x=845 whatever mono font the renderer resolves; the caret
// stands at x=858. Caret is magenta — the ink of action, and it blinks.
function wordmark(fg1, fg2, caret) {
  return `${svgOpen("0 0 940 140")}
  <text x="0" y="102" font-family="'IBM Plex Mono', Consolas, 'Cascadia Mono', 'DejaVu Sans Mono', monospace" font-size="96" letter-spacing="0">
    <tspan font-weight="600" fill="${fg1}" textLength="264" lengthAdjust="spacingAndGlyphs">klava</tspan><tspan font-weight="500" fill="${fg2}" x="264" textLength="581" lengthAdjust="spacingAndGlyphs">-nevinovata</tspan>
  </text>
  <rect x="858" y="26" width="24" height="88" fill="${caret}"/>
</svg>`;
}

// ---- job list ----
// Wordmark colors mirror the app tokens: deep magenta + violet-ink on kraft,
// bright magenta + cream-mut on the wall.
const jobs = [
  ["svg/logo.svg", "logo.png", 1024, logo],
  [
    "svg/wordmark-for-light-bg.svg",
    "wordmark/wordmark-for-light-bg.png",
    1880,
    wordmark("#b81261", "#3a2a7a", "#b81261"),
  ],
  [
    "svg/wordmark-for-dark-bg.svg",
    "wordmark/wordmark-for-dark-bg.png",
    1880,
    wordmark("#ff2f88", "#a99e83", "#ff2f88"),
  ],
];
for (const [state, file] of [
  ["idle", "tray_idle"],
  ["recording", "tray_recording"],
  ["transcribing", "tray_transcribing"],
]) {
  jobs.push([
    `svg/${file}.svg`,
    `tray/${file}.png`,
    64,
    trayMono(MONO_WHITE, state),
  ]);
  jobs.push([
    `svg/${file}_dark.svg`,
    `tray/${file}_dark.png`,
    64,
    trayMono(MONO_INK, state),
  ]);
}
for (const state of ["idle", "recording", "transcribing"]) {
  jobs.push([
    `svg/colored_${state}.svg`,
    `tray/colored_${state}.png`,
    64,
    trayColored(state),
  ]);
}

for (const [svgPath, pngPath, width, svg] of jobs) {
  writeFileSync(join(OUT, svgPath), svg);
  const r = new Resvg(svg, {
    fitTo: { mode: "width", value: width },
    font: { loadSystemFonts: true, defaultFontFamily: "Consolas" },
    background: "rgba(0,0,0,0)",
  });
  writeFileSync(join(OUT, pngPath), r.render().asPng());
  console.log(`ok ${pngPath} (${width}px)`);
}
console.log(`\ndone → ${OUT}`);

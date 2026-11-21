import assert from "node:assert/strict";
import {
  CURATED_ASR,
  BREEZE,
  TURBO,
  errorFor,
  groupModels,
  picksFor,
} from "./catalog";
import { makeGrading } from "./grading";

const breeze = CURATED_ASR.find((m) => m.id === BREEZE)!;
const turbo = CURATED_ASR.find((m) => m.id === TURBO)!;
for (const langs of [["ru"], ["en", "ru"]]) {
  const picks = picksFor(langs, () => true);
  const grading = makeGrading(langs, "cpu", picks);
  assert.equal(picks[0]?.model.id, BREEZE);
  assert.equal(picks[0]?.role, "pick");
  const groups = groupModels(langs, "cpu", CURATED_ASR, picks);
  assert.ok(!groups.partial.some((r) => r.model.id === BREEZE));
  for (const model of [breeze, { ...breeze, id: "breeze-asr" }]) {
    assert.deepEqual(grading.missing(model), []);
    assert.equal(grading.overall(model).label, "A−");
    assert.equal(grading.grade(model, "en").label, "A−");
    assert.equal(grading.grade(model, "ru").label, "A−");
    assert.equal(grading.grade(model, "ru").editorial, true);
  }
  for (const model of [turbo, { ...turbo, id: "turbo" }])
    assert.equal(grading.grade(model, "ru").label, "A−");
}
assert.equal(errorFor(breeze, "ru"), undefined);
assert.ok(
  !picksFor(["ru", "fr"], () => true).some((p) => p.model.id === BREEZE),
);
assert.ok(
  !picksFor(["ru"], (m) => m.id !== BREEZE).some((p) => p.model.id === BREEZE),
);

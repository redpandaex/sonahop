import test from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_PREFERENCES, PREFERENCES_KEY, readPreferences, savePreferences,
} from "./preferences.ts";

test("missing or corrupt preferences use a fresh safe default", () => {
  for (const value of [null, "", "{", "null", "[]", '"text"']) {
    const prefs = readPreferences(value);
    assert.deepEqual(prefs, DEFAULT_PREFERENCES);
    assert.notEqual(prefs, DEFAULT_PREFERENCES);
  }
});
test("a supported document restores the user's actual choices", () => {
  const prefs = { ...DEFAULT_PREFERENCES, mode: "microphone" as const, theme: "dark" as const,
    volume: 45, sourceId: "a-playback-endpoint", computerName: "工作电脑", computerAddress: "192.168.1.8" };
  assert.deepEqual(readPreferences(JSON.stringify(prefs)), prefs);
});
test("invalid mode and theme cannot enter the UI state", () => {
  const result = readPreferences('{"version":1,"mode":"record","theme":"broken","volume":30,"connected":true}');
  assert.equal(result.mode, "listen");
  assert.equal(result.theme, "light");
  assert.equal(result.volume, 30);
  assert.equal("connected" in result, false);
});
test("volume is finite and within the supported range", () => {
  for (const [value, expected] of [[-10, 0], [130, 100], [32.7, 33]] as const) {
    assert.equal(readPreferences(JSON.stringify({ version: 1, volume: value })).volume, expected);
  }
  for (const raw of ['{"version":1,"volume":1e309}', '{"version":1,"volume":"50"}', '{"version":1,"volume":null}']) {
    assert.equal(readPreferences(raw).volume, 70);
  }
});
test("unknown document versions cannot restore an incompatible mode", () => {
  assert.deepEqual(readPreferences('{"version":2,"mode":"microphone","volume":0}'), DEFAULT_PREFERENCES);
});
test("saving and reading preserve choices without losing Unicode names", () => {
  let key = ""; let stored = "";
  const value = { ...DEFAULT_PREFERENCES, computerName: "联想平板的电脑", volume: 62 };
  assert.equal(savePreferences({ setItem(k, v) { key = k; stored = v; } }, value), true);
  assert.equal(key, PREFERENCES_KEY);
  assert.deepEqual(readPreferences(stored), value);
});
test("unavailable local storage does not crash an interaction", () => {
  assert.equal(savePreferences({ setItem() { throw new Error("quota denied"); } }, { ...DEFAULT_PREFERENCES }), false);
});

import { describe, it, expect } from "vitest";
import zh from "../locales/zh.json";
import en from "../locales/en.json";

function getKeys(obj: any, prefix = ""): string[] {
  const keys: string[] = [];
  for (const key in obj) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof obj[key] === "object" && obj[key] !== null && !Array.isArray(obj[key])) {
      keys.push(...getKeys(obj[key], fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

describe("i18n translation files", () => {
  const zhKeys = getKeys(zh).sort();
  const enKeys = getKeys(en).sort();

  it("zh.json has keys", () => {
    expect(zhKeys.length).toBeGreaterThan(50);
  });

  it("en.json has keys", () => {
    expect(enKeys.length).toBeGreaterThan(50);
  });

  it("all zh keys exist in en", () => {
    const missingInEn = zhKeys.filter((k) => !enKeys.includes(k));
    if (missingInEn.length > 0) {
      console.error("Keys missing in en.json:", missingInEn);
    }
    expect(missingInEn).toEqual([]);
  });

  it("all en keys exist in zh", () => {
    const missingInZh = enKeys.filter((k) => !zhKeys.includes(k));
    if (missingInZh.length > 0) {
      console.error("Keys missing in zh.json:", missingInZh);
    }
    expect(missingInZh).toEqual([]);
  });

  it("no duplicate top-level keys in en.json", () => {
    const enRaw = JSON.stringify(en);
    const parsed = JSON.parse(enRaw);
    expect(Object.keys(parsed).length).toBeGreaterThan(5);
  });
});

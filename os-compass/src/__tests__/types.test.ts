import { describe, it, expect } from "vitest";
import { parseLanguages, LIFECYCLE_STATUS_LABELS, LIFECYCLE_STATUS_COLORS } from "../types";

describe("parseLanguages", () => {
  it("returns empty array for null", () => {
    expect(parseLanguages(null)).toEqual([]);
  });

  it("returns empty array for empty string", () => {
    expect(parseLanguages("")).toEqual([]);
  });

  it("parses JSON array", () => {
    expect(parseLanguages('["Rust","TypeScript"]')).toEqual(["Rust", "TypeScript"]);
  });

  it("parses JSON string", () => {
    expect(parseLanguages('"Python"')).toEqual(["Python"]);
  });

  it("falls back to raw string on invalid JSON", () => {
    expect(parseLanguages("Go")).toEqual(["Go"]);
  });

  it("trims whitespace on fallback", () => {
    expect(parseLanguages("  Java  ")).toEqual(["Java"]);
  });

  it("returns empty array for whitespace-only string", () => {
    expect(parseLanguages("   ")).toEqual([]);
  });

  it("filters non-string entries in array", () => {
    expect(parseLanguages('["Rust", 123, "Go"]')).toEqual(["Rust", "Go"]);
  });
});

describe("LIFECYCLE_STATUS_LABELS", () => {
  it("has all 4 statuses", () => {
    expect(Object.keys(LIFECYCLE_STATUS_LABELS)).toHaveLength(4);
  });

  it("contains TO_EXPLORE", () => {
    expect(LIFECYCLE_STATUS_LABELS.TO_EXPLORE).toBe("待探索");
  });

  it("contains IN_USE", () => {
    expect(LIFECYCLE_STATUS_LABELS.IN_USE).toBe("使用中");
  });
});

describe("LIFECYCLE_STATUS_COLORS", () => {
  it("has all 4 statuses", () => {
    expect(Object.keys(LIFECYCLE_STATUS_COLORS)).toHaveLength(4);
  });

  it("each value is a non-empty string", () => {
    for (const color of Object.values(LIFECYCLE_STATUS_COLORS)) {
      expect(typeof color).toBe("string");
      expect(color.length).toBeGreaterThan(0);
    }
  });
});

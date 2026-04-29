import { describe, expect, test } from "vitest";

import {
  isPointerNearPageTurnEdge,
  normalizeSelectedBookText,
  selectedTextInfographicPrompt,
} from "./book-selection";

describe("book text selection helpers", () => {
  test("only treats narrow outer strips as page-turn edges", () => {
    const bounds = { left: 100, right: 500 };

    expect(isPointerNearPageTurnEdge(112, bounds)).toBe(true);
    expect(isPointerNearPageTurnEdge(488, bounds)).toBe(true);
    expect(isPointerNearPageTurnEdge(260, bounds)).toBe(false);
    expect(isPointerNearPageTurnEdge(80, bounds)).toBe(false);
    expect(isPointerNearPageTurnEdge(520, bounds)).toBe(false);
  });

  test("keeps the edge strip proportional on narrow books", () => {
    const bounds = { left: 0, right: 120 };

    expect(isPointerNearPageTurnEdge(29, bounds, 80)).toBe(true);
    expect(isPointerNearPageTurnEdge(31, bounds, 80)).toBe(false);
    expect(isPointerNearPageTurnEdge(89, bounds, 80)).toBe(false);
    expect(isPointerNearPageTurnEdge(90, bounds, 80)).toBe(true);
  });

  test("normalizes and trims selected book text for the infographic prompt", () => {
    expect(normalizeSelectedBookText("  voltage\n\npushes   charges  ")).toBe(
      "voltage pushes charges",
    );
    expect(normalizeSelectedBookText("abcdef", 4)).toBe("abc...");

    const prompt = selectedTextInfographicPrompt(
      "water moves when pressure changes",
      "reef currents",
    );

    expect(prompt).toContain("reef currents");
    expect(prompt).toContain("water moves when pressure changes");
    expect(prompt).toContain("no tiny text");
  });
});

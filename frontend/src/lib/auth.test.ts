import { describe, expect, test } from "vitest";

import {
  authHeaders,
  normalizeAuthenticatedStudent,
  readStoredAuth,
  splitInterests,
} from "./auth";

describe("auth helpers", () => {
  test("builds bearer headers only when a token is present", () => {
    expect(authHeaders(null)).toEqual({});
    expect(authHeaders({ token: "", type: "local-demo" })).toEqual({});
    expect(authHeaders({ token: "local-demo:student-1", type: "local-demo" }))
      .toEqual({
        Authorization: "Bearer local-demo:student-1",
      });
  });

  test("splits signup interests into trimmed non-empty entries", () => {
    expect(splitInterests(" marine biology, drawing ,, puzzles ")).toEqual([
      "marine biology",
      "drawing",
      "puzzles",
    ]);
  });

  test("does not read browser storage during server-side execution", () => {
    expect(readStoredAuth()).toBeNull();
  });

  test("normalizes authenticated students with persisted XP defaults", () => {
    expect(
      normalizeAuthenticatedStudent({
        studentId: "student-1",
        displayName: "Mina",
        interests: ["marine biology", 12],
        suggestedTopics: ["reefs"],
      }),
    ).toMatchObject({
      studentId: "student-1",
      displayName: "Mina",
      interests: ["marine biology"],
      suggestedTopics: ["reefs"],
      xpTotal: 0,
    });

    expect(
      normalizeAuthenticatedStudent({
        studentId: "student-2",
        displayName: "Iko",
        xpTotal: 75.9,
      }),
    ).toMatchObject({
      studentId: "student-2",
      xpTotal: 75,
    });
  });
});

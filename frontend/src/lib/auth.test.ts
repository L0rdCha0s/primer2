import { describe, expect, test } from "vitest";

import { authHeaders, readStoredAuth, splitInterests } from "./auth";

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
});

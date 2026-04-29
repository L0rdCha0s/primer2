import { describe, expect, test } from "vitest";

import {
  coverageStatusLabel,
  normalizeReportCard,
  reportCardStats,
} from "./report-card";

describe("report-card normalization", () => {
  test("normalizes schema-stable report card payloads", () => {
    const report = normalizeReportCard({
      studentId: "student-123",
      displayName: "Mina",
      generatedAt: "2026-04-29T00:00:00Z",
      aiMode: "missing_openai_api_key",
      yearLevel: {
        code: "7",
        label: "Year 7",
        age: 12,
      },
      studentSummary: "You explored reef currents.",
      parentSummary: "Evidence summary for parent review.",
      learnedTopics: [
        {
          topic: "reef currents",
          levels: ["intuition", 7, "mechanism"],
          bestScore: 0.88,
          status: "passed",
          evidence: ["Named the cause.", 42],
          lastUpdated: "2026-04-29T00:00:00Z",
        },
      ],
      stagegateSummary: {
        totalAttempts: 1,
        passedAttempts: 1,
        averageScore: 0.88,
        latestAttempt: {
          topic: "reef currents",
          stageLevel: "intuition",
          score: 0.88,
          passed: true,
          feedback: "Level 2 unlocked.",
          masteryEvidence: ["Named the cause."],
          gaps: ["Add a transfer example."],
          submittedAt: "2026-04-29T00:00:00Z",
        },
        attempts: [],
      },
      curriculumCoverage: [
        {
          learningArea: "Science",
          strand: "Science Inquiry",
          yearLevel: "Year 7",
          referenceId: "primer-ac9-science-inquiry-understanding",
          referenceLabel: "Australian Curriculum v9.0 Science",
          sourceUrl: "https://example.test/science",
          status: "covered",
          evidenceTopics: ["reef currents"],
          evidenceCount: 1,
          averageScore: 0.88,
          parentNote: "Linked through causal explanation.",
        },
      ],
      strengths: ["Strong causal explanation."],
      growthAreas: ["Add transfer."],
      nextSteps: ["Try mechanism."],
      memoryHighlights: ["Learner likes diagrams."],
      sources: [{ label: "AC v9", url: "https://example.test" }],
    });

    expect(report).toMatchObject({
      studentId: "student-123",
      displayName: "Mina",
      aiMode: "missing_openai_api_key",
      yearLevel: { label: "Year 7", age: 12 },
      learnedTopics: [
        {
          topic: "reef currents",
          levels: ["intuition", "mechanism"],
          bestScore: 0.88,
          evidence: ["Named the cause."],
        },
      ],
      stagegateSummary: {
        totalAttempts: 1,
        passedAttempts: 1,
        averageScore: 0.88,
      },
      curriculumCoverage: [
        {
          learningArea: "Science",
          status: "covered",
          evidenceCount: 1,
        },
      ],
    });
  });

  test("handles empty report-card state without throwing", () => {
    expect(normalizeReportCard(null)).toBeNull();
    expect(reportCardStats(null)).toEqual({
      topicCount: 0,
      attemptCount: 0,
      passedAttemptCount: 0,
      coveredAreaCount: 0,
    });
  });

  test("summarizes curriculum coverage for display", () => {
    const report = normalizeReportCard({
      studentId: "student-123",
      yearLevel: { label: "Year 7" },
      stagegateSummary: {},
      curriculumCoverage: [
        { learningArea: "Science", status: "covered" },
        { learningArea: "English", status: "developing" },
        { learningArea: "Mathematics", status: "not_evidenced" },
      ],
    });

    expect(reportCardStats(report).coveredAreaCount).toBe(1);
    expect(coverageStatusLabel("covered")).toBe("Covered");
    expect(coverageStatusLabel("developing")).toBe("Developing");
    expect(coverageStatusLabel("not_evidenced")).toBe("Not evidenced");
  });
});

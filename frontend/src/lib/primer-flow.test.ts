import { describe, expect, test } from "vitest";

import {
  buildLessonStartBody,
  bookEndPageIndex,
  bookContentPageCount,
  defaultBookPageIndex,
  emptyStagegateResult,
  firstTopicHint,
  initialLesson,
  isBookContentPage,
  mergeMemoryGraph,
  normalizeBookState,
  normalizeLesson,
  normalizeMemories,
  normalizeMemoryGraph,
  normalizeStagegateResult,
  stagesForStagegate,
  staticBookPageCount,
  visibleBookContentPages,
  type StudentMemoryGraph,
} from "./primer-flow";

const learner = {
  studentId: "student-123",
  interests: ["marine biology", "drawing"],
};

describe("lesson start flow", () => {
  test("asks the backend to choose a profile-based opening path when topic is blank", () => {
    expect(buildLessonStartBody(learner, "  ")).toEqual({
      studentId: "student-123",
      question:
        "Use my signup biography, interests, and progress to choose the most engaging first learning path for me.",
    });
  });

  test("sends trimmed explicit topics without changing the learner id", () => {
    expect(buildLessonStartBody(learner, " lightning ")).toEqual({
      studentId: "student-123",
      topic: "lightning",
      question: "I want to explore lightning. Guide me at my current level.",
    });
  });

  test("uses the first learner interest as the opening fallback topic hint", () => {
    expect(firstTopicHint(learner)).toBe("marine biology");
    expect(firstTopicHint({ interests: [] })).toBe("personalized starting point");
  });
});

describe("lesson and stagegate normalization", () => {
  test("initial lesson placeholder avoids product-personified page copy", () => {
    const studentFacingText = [
      initialLesson.communicationStyle,
      initialLesson.storyScene,
      initialLesson.plainExplanation,
      initialLesson.analogy,
      initialLesson.checkForUnderstanding,
    ].join(" ");

    expect(studentFacingText).not.toMatch(/the primer|the book|the page/i);
  });

  test("preserves schema-stable lesson data and clamps unknown stages", () => {
    const lesson = normalizeLesson(
      {
        topic: "reef currents",
        stageLevel: "expert",
        communicationStyle: "visual",
        storyScene: "A reef gate opens.",
        plainExplanation: "Currents move because of forces.",
        analogy: "A map of arrows.",
        checkForUnderstanding: "What causes the current?",
        suggestedTopics: ["waves", 7, "tides"],
        stagegatePrompt: "Explain it.",
        infographicPrompt: "Draw it.",
        keyTerms: [
          { term: "Current", definition: "Moving water." },
          { term: "", definition: "Ignored." },
          { term: "Force" },
        ],
        aiMode: "missing_openai_api_key",
      },
      "fallback",
    );

    expect(lesson).toMatchObject({
      topic: "reef currents",
      stageLevel: "intuition",
      suggestedTopics: ["waves", "tides"],
      keyTerms: [{ term: "Current", definition: "Moving water." }],
      aiMode: "missing_openai_api_key",
    });
  });

  test("normalizes stagegate pass results and unlocks mechanism stage", () => {
    const result = normalizeStagegateResult({
      passed: true,
      score: 0.88,
      rubric: {
        accuracy: 0.9,
        causalReasoning: 0.8,
        vocabulary: 0.85,
        transfer: 0.95,
      },
      masteryEvidence: ["Named the cause."],
      gaps: ["Practice the vocabulary."],
      feedbackToStudent: "Level 2 unlocked.",
      nextLevelUnlocked: "mechanism",
      newMemories: [
        {
          assertionId: "fact-1",
          memoryType: "knowledge",
          content: "Learner explained cause and effect.",
          tags: ["mastery", 12],
        },
      ],
    });

    expect(result.passed).toBe(true);
    expect(result.newMemories).toEqual([
      {
        id: "fact-1",
        assertionId: "fact-1",
        type: "knowledge",
        content: "Learner explained cause and effect.",
        tags: ["mastery"],
        subject: undefined,
        predicate: undefined,
        validFrom: undefined,
        validTo: undefined,
        knownFrom: undefined,
        knownTo: undefined,
        source: undefined,
      },
    ]);

    expect(stagesForStagegate(result.passed).map((stage) => stage.status))
      .toEqual(["passed", "available", "locked"]);
  });

  test("falls back to the empty stagegate result for invalid payloads", () => {
    expect(normalizeStagegateResult(null)).toBe(emptyStagegateResult);
  });
});

describe("memory contract normalization", () => {
  test("accepts backend snake_case and camelCase memory type fields", () => {
    expect(
      normalizeMemories([
        {
          assertionId: "a1",
          memory_type: "preference",
          content: "Learner likes diagrams.",
          tags: ["style", 100],
        },
        {
          memoryType: "misconception",
          content: "Learner confuses voltage and current.",
          tags: ["electricity"],
        },
        { memoryType: "unknown", content: "Defaults to knowledge." },
        { memoryType: "knowledge" },
      ]),
    ).toMatchObject([
      {
        id: "a1",
        type: "preference",
        content: "Learner likes diagrams.",
        tags: ["style"],
      },
      {
        type: "misconception",
        content: "Learner confuses voltage and current.",
        tags: ["electricity"],
      },
      {
        type: "knowledge",
        content: "Defaults to knowledge.",
        tags: [],
      },
    ]);
  });

  test("normalizes graph payloads and merges walked nodes without duplicates", () => {
    const currentGraph = normalizeMemoryGraph({
      studentId: "student-123",
      rootNodeId: "entity-root",
      selectedNodeId: "entity-root",
      nodes: [
        {
          id: "entity-root",
          nodeType: "entity",
          kind: "student",
          label: "Mina",
          expanded: true,
          factCount: 1,
        },
      ],
      edges: [
        {
          id: "edge-1",
          source: "entity-root",
          target: "value-1",
          label: "likes",
          content: "Learner likes diagrams.",
          memoryType: "preference",
          confidence: 0.9,
        },
      ],
    }) as StudentMemoryGraph;

    const nextGraph = normalizeMemoryGraph({
      studentId: "student-123",
      rootNodeId: "entity-root",
      selectedNodeId: "value-1",
      nodes: [
        {
          id: "entity-root",
          label: "Mina",
          expanded: false,
          factCount: 4,
        },
        {
          id: "value-1",
          nodeType: "value",
          kind: "preference",
          label: "Likes diagrams",
          expanded: true,
          factCount: 2,
        },
      ],
      edges: [
        {
          id: "edge-1",
          source: "entity-root",
          target: "value-1",
        },
        {
          id: "edge-2",
          source: "value-1",
          target: "entity-topic",
        },
      ],
    }) as StudentMemoryGraph;

    const merged = mergeMemoryGraph(currentGraph, nextGraph);

    expect(merged.rootNodeId).toBe("entity-root");
    expect(merged.selectedNodeId).toBe("value-1");
    expect(merged.nodes).toHaveLength(2);
    expect(merged.edges.map((edge) => edge.id)).toEqual(["edge-1", "edge-2"]);
    expect(merged.nodes.find((node) => node.id === "entity-root")).toMatchObject({
      expanded: true,
      factCount: 4,
    });
  });
});

describe("persisted book contract normalization", () => {
  test("calculates the end page after persisted lesson pages", () => {
    expect(staticBookPageCount).toBe(10);
    expect(bookEndPageIndex(0)).toBe(9);
    expect(bookEndPageIndex(1)).toBe(10);
    expect(bookEndPageIndex(4)).toBe(13);
  });

  test("keeps stagegate attempts out of appended content pages", () => {
    const pages = [
      { kind: "lesson" },
      { kind: "stagegate" },
      { kind: "stagegate" },
    ];

    expect(pages.filter(isBookContentPage)).toEqual([{ kind: "lesson" }]);
    expect(bookContentPageCount(pages)).toBe(1);
    expect(defaultBookPageIndex(pages)).toBe(9);
    expect(defaultBookPageIndex([{ kind: "lesson" }, { kind: "infographic" }]))
      .toBe(11);
  });

  test("keeps all persisted lesson and infographic pages visible in order", () => {
    const book = normalizeBookState({
      studentId: "student-123",
      bookId: "book-1",
      lessons: [
        {
          lessonId: "lesson-2",
          topic: "basketball arcs",
          position: 2,
          lesson: { topic: "basketball arcs" },
          pages: [
            {
              pageId: "lesson-page-2",
              lessonId: "lesson-2",
              kind: "lesson",
              topic: "basketball arcs",
              position: 1,
              payload: { lesson: { topic: "basketball arcs" } },
            },
          ],
        },
        {
          lessonId: "lesson-1",
          topic: "reef currents",
          position: 1,
          lesson: { topic: "reef currents" },
          pages: [
            {
              pageId: "stagegate-1",
              lessonId: "lesson-1",
              kind: "stagegate",
              position: 3,
              payload: {},
            },
            {
              pageId: "lesson-page-1",
              lessonId: "lesson-1",
              kind: "lesson",
              topic: "reef currents",
              position: 1,
              payload: { lesson: { topic: "reef currents" } },
            },
            {
              pageId: "diagram-1",
              lessonId: "lesson-1",
              kind: "infographic",
              topic: "reef currents",
              position: 2,
              payload: { artifact: { generated: false } },
            },
          ],
        },
      ],
    });

    const visiblePages = visibleBookContentPages(book?.lessons ?? []);
    expect(visiblePages.map((page) => ({
      pageId: page.pageId,
      kind: page.kind,
      topic: page.topic,
    }))).toEqual([
      { pageId: "lesson-page-1", kind: "lesson", topic: "reef currents" },
      { pageId: "diagram-1", kind: "infographic", topic: "reef currents" },
      { pageId: "lesson-page-2", kind: "lesson", topic: "basketball arcs" },
    ]);
    expect(bookContentPageCount(visiblePages)).toBe(3);
    expect(defaultBookPageIndex(visiblePages)).toBe(12);
  });

  test("normalizes persisted book lessons, pages, and latest interaction state", () => {
    const book = normalizeBookState({
      studentId: "student-123",
      bookId: "book-1",
      currentLessonId: "lesson-1",
      currentLesson: {
        topic: "reef currents",
        stageLevel: "intuition",
      },
      latestInfographic: {
        generated: true,
        model: "gpt-image-2",
      },
      latestStagegate: {
        passed: true,
        score: 0.9,
      },
      latestAnswer: "Forces push water.",
      hasPassedStagegate: true,
      lessons: [
        {
          lessonId: "lesson-1",
          topic: "reef currents",
          stageLevel: "intuition",
          position: 1,
          lesson: { topic: "reef currents" },
          createdAt: "2026-04-29T00:00:00Z",
          updatedAt: "2026-04-29T00:00:00Z",
          pages: [
            {
              pageId: "page-1",
              lessonId: "lesson-1",
              kind: "lesson",
              topic: "reef currents",
              stageLevel: "intuition",
              position: 1,
              payload: { lesson: { topic: "reef currents" } },
              createdAt: "2026-04-29T00:00:00Z",
            },
            {
              pageId: "page-bad",
              lessonId: "lesson-1",
              kind: "lesson",
              position: "2",
              payload: {},
            },
          ],
        },
      ],
    });

    expect(book).toMatchObject({
      studentId: "student-123",
      bookId: "book-1",
      hasPassedStagegate: true,
      latestAnswer: "Forces push water.",
      currentLessonId: "lesson-1",
      lessons: [
        {
          lessonId: "lesson-1",
          topic: "reef currents",
          stageLevel: "intuition",
          position: 1,
          pages: [
            {
              pageId: "page-1",
              kind: "lesson",
              topic: "reef currents",
              stageLevel: "intuition",
              position: 1,
            },
          ],
        },
      ],
    });
  });
});

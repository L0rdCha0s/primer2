import type { AuthenticatedStudent } from "./auth";

export type StageLevel = "intuition" | "mechanism" | "transfer";

export type Stage = {
  level: StageLevel;
  title: string;
  status: "available" | "locked" | "passed";
  description: string;
};

export type StudentMemory = {
  id: string;
  type: "preference" | "knowledge" | "misconception" | "interest" | "history";
  content: string;
  tags: string[];
  assertionId?: string;
  subject?: string;
  predicate?: string;
  validFrom?: string;
  validTo?: string;
  knownFrom?: string;
  knownTo?: string;
  source?: string;
};

export type StagegateResult = {
  passed: boolean;
  score: number;
  rubric: {
    accuracy: number;
    causalReasoning: number;
    vocabulary: number;
    transfer: number;
  };
  masteryEvidence: string[];
  gaps: string[];
  feedbackToStudent: string;
  nextLevelUnlocked?: "mechanism" | "transfer" | "complete";
  newMemories?: StudentMemory[];
};

export type PrimerLesson = {
  topic: string;
  stageLevel: StageLevel;
  communicationStyle: string;
  storyScene: string;
  plainExplanation: string;
  analogy: string;
  checkForUnderstanding: string;
  suggestedTopics: string[];
  stagegatePrompt: string;
  infographicPrompt: string;
  keyTerms: Array<{
    term: string;
    definition: string;
  }>;
  aiMode?: string;
  model?: string;
};

export type StudentBookEntry = {
  entryId: string;
  kind: "lesson" | "infographic" | "stagegate" | string;
  topic?: string;
  stageLevel?: string;
  position: number;
  payload: Record<string, unknown>;
  createdAt: string;
};

export type StudentBookState = {
  studentId: string;
  bookId: string;
  entries: StudentBookEntry[];
  activeLesson?: unknown;
  latestInfographic?: unknown;
  latestStagegate?: unknown;
  latestAnswer?: string;
  hasPassedStagegate: boolean;
};

export const staticBookPageCount = 12;

export function bookEndPageIndex(entryCount: number): number {
  const safeEntryCount = Number.isFinite(entryCount)
    ? Math.max(0, Math.floor(entryCount))
    : 0;

  return staticBookPageCount + safeEntryCount - 1;
}

export function isBookContentEntry(
  entry: Pick<StudentBookEntry, "kind">,
): entry is Pick<StudentBookEntry, "kind"> & {
  kind: "lesson" | "infographic";
} {
  return entry.kind === "lesson" || entry.kind === "infographic";
}

export function bookContentEntryCount(
  entries: Array<Pick<StudentBookEntry, "kind">>,
): number {
  return entries.filter(isBookContentEntry).length;
}

export function defaultBookPageIndex(
  entries: Array<Pick<StudentBookEntry, "kind">>,
): number {
  const latestEntry = entries.at(-1);
  if (latestEntry?.kind === "stagegate") {
    return staticBookPageCount - 1;
  }

  return bookEndPageIndex(bookContentEntryCount(entries));
}

type BackendMemory = {
  assertionId?: string;
  memory_type?: unknown;
  memoryType?: unknown;
  content?: unknown;
  tags?: unknown;
  subject?: unknown;
  predicate?: unknown;
  validFrom?: unknown;
  validTo?: unknown;
  knownFrom?: unknown;
  knownTo?: unknown;
  source?: unknown;
};

export type MemoryGraphNodeRecord = {
  id: string;
  nodeType: string;
  kind: string;
  label: string;
  summary?: string | null;
  expanded: boolean;
  factCount: number;
};

export type MemoryGraphEdgeRecord = {
  id: string;
  source: string;
  target: string;
  label: string;
  assertionId: string;
  predicate: string;
  content: string;
  memoryType: string;
  confidence: number;
  observedAt: string;
  validFrom?: string | null;
  knownFrom?: string | null;
};

export type StudentMemoryGraph = {
  studentId: string;
  rootNodeId: string;
  selectedNodeId: string;
  nodes: MemoryGraphNodeRecord[];
  edges: MemoryGraphEdgeRecord[];
  validAsOf: string;
  knownAsOf: string;
};

export type LessonStartPayload = {
  aiMode?: string;
  book?: unknown;
  error?: string;
  lesson?: unknown;
  student?: AuthenticatedStudent;
  studentId?: string;
};

export type LessonStartBody = {
  studentId: string;
  topic?: string;
  question: string;
};

export const baseStages: Stage[] = [
  {
    level: "intuition",
    title: "Level 1: Intuition",
    status: "available",
    description: "Explain the idea in plain language.",
  },
  {
    level: "mechanism",
    title: "Level 2: Mechanism",
    status: "locked",
    description: "Order the causal process and name the parts.",
  },
  {
    level: "transfer",
    title: "Level 3: Transfer",
    status: "locked",
    description: "Apply the idea to a new case.",
  },
];

export const initialLesson: PrimerLesson = {
  topic: "personalized starting point",
  stageLevel: "intuition",
  communicationStyle: "Preparing a profile-aware story path.",
  storyScene:
    "Profile interests and biography details will shape the first learning path.",
  plainExplanation:
    "A starting lesson will appear here after the backend chooses a topic from the student's signup biography and interests.",
  analogy:
    "The first path is selected from the learner's own profile instead of a prewritten demo script.",
  checkForUnderstanding:
    "Once the opening lesson is ready, a check-for-understanding question will appear here.",
  suggestedTopics: [],
  stagegatePrompt:
    "Explain the most important idea in your own words, then connect it to one example from your life.",
  infographicPrompt:
    "Create an age-appropriate infographic for the student's generated starter lesson.",
  keyTerms: [],
};

export const emptyStagegateResult: StagegateResult = {
  passed: false,
  score: 0,
  rubric: {
    accuracy: 0,
    causalReasoning: 0,
    vocabulary: 0,
    transfer: 0,
  },
  masteryEvidence: [],
  gaps: [],
  feedbackToStudent: "Submit the stagegate to get feedback.",
  nextLevelUnlocked: undefined,
  newMemories: [],
};

export function buildLessonStartBody(
  learner: Pick<AuthenticatedStudent, "studentId">,
  nextTopic?: string,
): LessonStartBody {
  const cleanTopic = nextTopic?.trim();
  const body: LessonStartBody = {
    studentId: learner.studentId,
    question: cleanTopic
      ? `I want to explore ${cleanTopic}. Guide me at my current level.`
      : "Use my signup biography, interests, and progress to choose the most engaging first learning path for me.",
  };

  if (cleanTopic) {
    body.topic = cleanTopic;
  }

  return body;
}

export function firstTopicHint(learner: Pick<AuthenticatedStudent, "interests">) {
  return learner.interests[0] ?? "personalized starting point";
}

export function stagesForStagegate(hasPassedStagegate: boolean): Stage[] {
  return baseStages.map((stage) => {
    if (!hasPassedStagegate) {
      return { ...stage };
    }

    if (stage.level === "intuition") {
      return { ...stage, status: "passed" };
    }

    if (stage.level === "mechanism") {
      return { ...stage, status: "available" };
    }

    return { ...stage };
  });
}

export function mergeMemoryGraph(
  currentGraph: StudentMemoryGraph | null,
  nextGraph: StudentMemoryGraph,
): StudentMemoryGraph {
  if (!currentGraph) {
    return nextGraph;
  }

  const nodes = new Map(
    currentGraph.nodes.map((node) => [node.id, node] as const),
  );
  const edges = new Map(
    currentGraph.edges.map((edge) => [edge.id, edge] as const),
  );
  for (const node of nextGraph.nodes) {
    const existing = nodes.get(node.id);
    nodes.set(node.id, {
      ...existing,
      ...node,
      expanded: Boolean(existing?.expanded || node.expanded),
      factCount: Math.max(existing?.factCount ?? 0, node.factCount),
    });
  }
  for (const edge of nextGraph.edges) {
    edges.set(edge.id, edge);
  }

  return {
    ...nextGraph,
    rootNodeId: currentGraph.rootNodeId,
    nodes: [...nodes.values()],
    edges: [...edges.values()],
  };
}

export function normalizeMemoryGraph(value: unknown): StudentMemoryGraph | null {
  const record = asRecord(value);
  if (!record) {
    return null;
  }

  const nodes = Array.isArray(record.nodes)
    ? record.nodes
        .map(normalizeMemoryGraphNode)
        .filter((node): node is MemoryGraphNodeRecord => node !== null)
    : [];
  const edges = Array.isArray(record.edges)
    ? record.edges
        .map(normalizeMemoryGraphEdge)
        .filter((edge): edge is MemoryGraphEdgeRecord => edge !== null)
    : [];
  const studentId = stringField(record, "studentId");
  const rootNodeId = stringField(record, "rootNodeId");
  const selectedNodeId = stringField(record, "selectedNodeId") ?? rootNodeId;
  if (!studentId || !rootNodeId || !selectedNodeId) {
    return null;
  }

  return {
    studentId,
    rootNodeId,
    selectedNodeId,
    nodes,
    edges,
    validAsOf: stringField(record, "validAsOf") ?? "",
    knownAsOf: stringField(record, "knownAsOf") ?? "",
  };
}

function normalizeMemoryGraphNode(value: unknown): MemoryGraphNodeRecord | null {
  const record = asRecord(value);
  const id = stringField(record, "id");
  const label = stringField(record, "label");
  if (!id || !label) {
    return null;
  }

  return {
    id,
    nodeType: stringField(record, "nodeType") ?? "entity",
    kind: stringField(record, "kind") ?? "memory",
    label,
    summary: stringField(record, "summary"),
    expanded: booleanField(record, "expanded") ?? false,
    factCount: numberField(record, "factCount") ?? 0,
  };
}

function normalizeMemoryGraphEdge(value: unknown): MemoryGraphEdgeRecord | null {
  const record = asRecord(value);
  const id = stringField(record, "id");
  const source = stringField(record, "source");
  const target = stringField(record, "target");
  if (!id || !source || !target) {
    return null;
  }

  return {
    id,
    source,
    target,
    label: stringField(record, "label") ?? "",
    assertionId: stringField(record, "assertionId") ?? id,
    predicate: stringField(record, "predicate") ?? "",
    content: stringField(record, "content") ?? "",
    memoryType: stringField(record, "memoryType") ?? "memory",
    confidence: numberField(record, "confidence") ?? 0,
    observedAt: stringField(record, "observedAt") ?? "",
    validFrom: stringField(record, "validFrom"),
    knownFrom: stringField(record, "knownFrom"),
  };
}

export function normalizeLesson(
  value: unknown,
  fallbackTopic: string,
): PrimerLesson {
  const record = asRecord(value);
  if (!record) {
    return { ...initialLesson, topic: fallbackTopic };
  }

  const stageLevel = stringField(record, "stageLevel");
  const normalizedStage =
    stageLevel === "mechanism" || stageLevel === "transfer"
      ? stageLevel
      : "intuition";

  return {
    topic: stringField(record, "topic") ?? fallbackTopic,
    stageLevel: normalizedStage,
    communicationStyle:
      stringField(record, "communicationStyle") ??
      initialLesson.communicationStyle,
    storyScene: stringField(record, "storyScene") ?? initialLesson.storyScene,
    plainExplanation:
      stringField(record, "plainExplanation") ?? initialLesson.plainExplanation,
    analogy: stringField(record, "analogy") ?? initialLesson.analogy,
    checkForUnderstanding:
      stringField(record, "checkForUnderstanding") ??
      initialLesson.checkForUnderstanding,
    suggestedTopics:
      stringArrayField(record, "suggestedTopics") ??
      initialLesson.suggestedTopics,
    stagegatePrompt:
      stringField(record, "stagegatePrompt") ?? initialLesson.stagegatePrompt,
    infographicPrompt:
      stringField(record, "infographicPrompt") ??
      initialLesson.infographicPrompt,
    keyTerms: keyTermsField(record, "keyTerms") ?? initialLesson.keyTerms,
    aiMode: stringField(record, "aiMode"),
    model: stringField(record, "model"),
  };
}

export function normalizeStagegateResult(value: unknown): StagegateResult {
  const record = asRecord(value);
  if (!record) {
    return emptyStagegateResult;
  }

  const fallback = asRecord(record.fallback);
  if (fallback) {
    return normalizeStagegateResult(fallback);
  }

  const rubric = asRecord(record.rubric);

  return {
    passed: booleanField(record, "passed") ?? false,
    score: numberField(record, "score") ?? 0,
    rubric: {
      accuracy: numberField(rubric, "accuracy") ?? 0,
      causalReasoning: numberField(rubric, "causalReasoning") ?? 0,
      vocabulary: numberField(rubric, "vocabulary") ?? 0,
      transfer: numberField(rubric, "transfer") ?? 0,
    },
    masteryEvidence: stringArrayField(record, "masteryEvidence") ?? [],
    gaps: stringArrayField(record, "gaps") ?? [],
    feedbackToStudent:
      stringField(record, "feedbackToStudent") ??
      "The answer could not be graded yet.",
    nextLevelUnlocked: nextLevelField(record, "nextLevelUnlocked"),
    newMemories: normalizeMemories(record.newMemories) ?? [],
  };
}

export function normalizeBookState(value: unknown): StudentBookState | null {
  const record = asRecord(value);
  if (!record) {
    return null;
  }

  const studentId = stringField(record, "studentId");
  const bookId = stringField(record, "bookId");
  if (!studentId || !bookId) {
    return null;
  }

  const entries = Array.isArray(record.entries)
    ? record.entries
        .map(normalizeBookEntry)
        .filter((entry): entry is StudentBookEntry => entry !== null)
    : [];

  return {
    studentId,
    bookId,
    entries,
    activeLesson: record.activeLesson,
    latestInfographic: record.latestInfographic,
    latestStagegate: record.latestStagegate,
    latestAnswer: stringField(record, "latestAnswer"),
    hasPassedStagegate: booleanField(record, "hasPassedStagegate") ?? false,
  };
}

function normalizeBookEntry(value: unknown): StudentBookEntry | null {
  const record = asRecord(value);
  const entryId = stringField(record, "entryId");
  const kind = stringField(record, "kind");
  const position = numberField(record, "position");
  if (!entryId || !kind || typeof position !== "number") {
    return null;
  }

  return {
    entryId,
    kind,
    topic: stringField(record, "topic"),
    stageLevel: stringField(record, "stageLevel"),
    position,
    payload: asRecord(record?.payload) ?? {},
    createdAt: stringField(record, "createdAt") ?? "",
  };
}

export function normalizeMemories(value: unknown): StudentMemory[] | null {
  if (!Array.isArray(value)) {
    return null;
  }

  const normalized = value
    .map((item, index): StudentMemory | null => {
      const memory = asRecord(item) as BackendMemory | null;
      if (!memory) {
        return null;
      }

      const content = typeof memory?.content === "string" ? memory.content : "";
      if (!content) {
        return null;
      }

      const normalizedMemory: StudentMemory = {
        id:
          stringValue(memory.assertionId) ??
          `${content}-${index}`,
        type: memoryTypeField(memory.memory_type ?? memory.memoryType),
        content,
        tags: Array.isArray(memory.tags)
          ? memory.tags.filter((tag): tag is string => typeof tag === "string")
          : [],
        assertionId: stringValue(memory.assertionId),
        subject: stringValue(memory.subject),
        predicate: stringValue(memory.predicate),
        validFrom: stringValue(memory.validFrom),
        validTo: stringValue(memory.validTo),
        knownFrom: stringValue(memory.knownFrom),
        knownTo: stringValue(memory.knownTo),
        source: stringValue(memory.source),
      };
      return normalizedMemory;
    })
    .filter((memory): memory is StudentMemory => memory !== null);

  return normalized.length > 0 ? normalized : null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object"
    ? (value as Record<string, unknown>)
    : null;
}

function stringField(
  record: Record<string, unknown> | null,
  key: string,
): string | undefined {
  const value = record?.[key];
  return stringValue(value);
}

function stringValue(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function booleanField(
  record: Record<string, unknown> | null,
  key: string,
): boolean | undefined {
  const value = record?.[key];
  return typeof value === "boolean" ? value : undefined;
}

function numberField(
  record: Record<string, unknown> | null,
  key: string,
): number | undefined {
  const value = record?.[key];
  return typeof value === "number" ? value : undefined;
}

function stringArrayField(
  record: Record<string, unknown> | null,
  key: string,
): string[] | undefined {
  const value = record?.[key];
  if (!Array.isArray(value)) {
    return undefined;
  }

  return value.filter((item): item is string => typeof item === "string");
}

function keyTermsField(
  record: Record<string, unknown> | null,
  key: string,
): PrimerLesson["keyTerms"] | undefined {
  const value = record?.[key];
  if (!Array.isArray(value)) {
    return undefined;
  }

  const terms = value
    .map((item) => {
      const term = asRecord(item);
      const termName = stringField(term, "term");
      const definition = stringField(term, "definition");
      return termName && definition ? { term: termName, definition } : null;
    })
    .filter(
      (term): term is PrimerLesson["keyTerms"][number] => term !== null,
    );

  return terms.length > 0 ? terms : undefined;
}

function nextLevelField(
  record: Record<string, unknown>,
  key: string,
): StagegateResult["nextLevelUnlocked"] {
  const value = record[key];
  return value === "mechanism" || value === "transfer" || value === "complete"
    ? value
    : undefined;
}

function memoryTypeField(value: unknown): StudentMemory["type"] {
  return value === "preference" ||
    value === "knowledge" ||
    value === "misconception" ||
    value === "interest" ||
    value === "history"
    ? value
    : "knowledge";
}

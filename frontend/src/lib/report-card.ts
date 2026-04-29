import {
  type AuthSession,
  type AuthenticatedStudent,
  apiBaseUrl,
  authHeaders,
} from "./auth";

export type CurriculumYearLevel = {
  code: string;
  label: string;
  age?: number | null;
  note?: string | null;
};

export type ReportLearnedTopic = {
  topic: string;
  levels: string[];
  bestScore: number;
  status: string;
  evidence: string[];
  lastUpdated: string;
};

export type StagegateAttempt = {
  topic: string;
  stageLevel: string;
  score: number;
  passed: boolean;
  feedback: string;
  masteryEvidence: string[];
  gaps: string[];
  submittedAt: string;
};

export type StagegateSummary = {
  totalAttempts: number;
  passedAttempts: number;
  averageScore: number;
  latestAttempt?: StagegateAttempt | null;
  attempts: StagegateAttempt[];
};

export type CurriculumCoverage = {
  learningArea: string;
  strand: string;
  yearLevel: string;
  referenceId: string;
  referenceLabel: string;
  sourceUrl: string;
  status: "covered" | "developing" | "not_evidenced" | string;
  evidenceTopics: string[];
  evidenceCount: number;
  averageScore: number;
  parentNote: string;
};

export type CurriculumSource = {
  label: string;
  url: string;
};

export type StudentReportCard = {
  studentId: string;
  displayName: string;
  generatedAt: string;
  aiMode: string;
  model?: string | null;
  narrativeError?: string | null;
  yearLevel: CurriculumYearLevel;
  studentSummary: string;
  parentSummary: string;
  learnedTopics: ReportLearnedTopic[];
  stagegateSummary: StagegateSummary;
  curriculumCoverage: CurriculumCoverage[];
  strengths: string[];
  growthAreas: string[];
  nextSteps: string[];
  memoryHighlights: string[];
  sources: CurriculumSource[];
};

export type ReportCardPayload = {
  error?: string;
  reportCard?: unknown;
  studentId?: string;
};

export type ReportCardStats = {
  topicCount: number;
  attemptCount: number;
  passedAttemptCount: number;
  coveredAreaCount: number;
};

export async function fetchStudentReportCard(
  learner: AuthenticatedStudent,
  session: AuthSession | null,
  signal?: AbortSignal,
): Promise<StudentReportCard | null> {
  const response = await fetch(
    `${apiBaseUrl}/api/students/${encodeURIComponent(
      learner.studentId,
    )}/report-card`,
    {
      method: "GET",
      headers: authHeaders(session),
      signal,
    },
  );
  const payload = (await response.json()) as ReportCardPayload;
  if (payload.error || !payload.reportCard) {
    return null;
  }

  return normalizeReportCard(payload.reportCard);
}

export function normalizeReportCard(value: unknown): StudentReportCard | null {
  const record = asRecord(value);
  const studentId = stringField(record, "studentId");
  if (!studentId) {
    return null;
  }

  return {
    studentId,
    displayName: stringField(record, "displayName") ?? studentId,
    generatedAt: stringField(record, "generatedAt") ?? "",
    aiMode: stringField(record, "aiMode") ?? "deterministic_fallback",
    model: stringField(record, "model") ?? null,
    narrativeError: stringField(record, "narrativeError") ?? null,
    yearLevel: normalizeYearLevel(record?.yearLevel),
    studentSummary:
      stringField(record, "studentSummary") ??
      "Complete a lesson and stagegate to build this report card.",
    parentSummary:
      stringField(record, "parentSummary") ??
      "No report-card evidence is available yet.",
    learnedTopics: arrayField(record, "learnedTopics")
      .map(normalizeLearnedTopic)
      .filter((topic): topic is ReportLearnedTopic => topic !== null),
    stagegateSummary: normalizeStagegateSummary(record?.stagegateSummary),
    curriculumCoverage: arrayField(record, "curriculumCoverage")
      .map(normalizeCurriculumCoverage)
      .filter((item): item is CurriculumCoverage => item !== null),
    strengths: stringArrayField(record, "strengths"),
    growthAreas: stringArrayField(record, "growthAreas"),
    nextSteps: stringArrayField(record, "nextSteps"),
    memoryHighlights: stringArrayField(record, "memoryHighlights"),
    sources: arrayField(record, "sources")
      .map(normalizeSource)
      .filter((source): source is CurriculumSource => source !== null),
  };
}

export function reportCardStats(reportCard: StudentReportCard | null): ReportCardStats {
  if (!reportCard) {
    return {
      topicCount: 0,
      attemptCount: 0,
      passedAttemptCount: 0,
      coveredAreaCount: 0,
    };
  }

  return {
    topicCount: reportCard.learnedTopics.length,
    attemptCount: reportCard.stagegateSummary.totalAttempts,
    passedAttemptCount: reportCard.stagegateSummary.passedAttempts,
    coveredAreaCount: reportCard.curriculumCoverage.filter(
      (coverage) => coverage.status === "covered",
    ).length,
  };
}

export function coverageStatusLabel(status: string): string {
  switch (status) {
    case "covered":
      return "Covered";
    case "developing":
      return "Developing";
    case "not_evidenced":
      return "Not evidenced";
    default:
      return status || "Unknown";
  }
}

function normalizeYearLevel(value: unknown): CurriculumYearLevel {
  const record = asRecord(value);

  return {
    code: stringField(record, "code") ?? "",
    label: stringField(record, "label") ?? "Year level pending",
    age: numberField(record, "age") ?? null,
    note: stringField(record, "note") ?? null,
  };
}

function normalizeLearnedTopic(value: unknown): ReportLearnedTopic | null {
  const record = asRecord(value);
  const topic = stringField(record, "topic");
  if (!topic) {
    return null;
  }

  return {
    topic,
    levels: stringArrayField(record, "levels"),
    bestScore: normalizedScore(record?.bestScore),
    status: stringField(record, "status") ?? "exploring",
    evidence: stringArrayField(record, "evidence"),
    lastUpdated: stringField(record, "lastUpdated") ?? "",
  };
}

function normalizeStagegateSummary(value: unknown): StagegateSummary {
  const record = asRecord(value);
  const attempts = arrayField(record, "attempts")
    .map(normalizeStagegateAttempt)
    .filter((attempt): attempt is StagegateAttempt => attempt !== null);

  return {
    totalAttempts:
      Math.max(0, Math.floor(numberField(record, "totalAttempts") ?? attempts.length)),
    passedAttempts: Math.max(
      0,
      Math.floor(
        numberField(record, "passedAttempts") ??
          attempts.filter((attempt) => attempt.passed).length,
      ),
    ),
    averageScore: normalizedScore(record?.averageScore),
    latestAttempt: normalizeStagegateAttempt(record?.latestAttempt),
    attempts,
  };
}

function normalizeStagegateAttempt(value: unknown): StagegateAttempt | null {
  const record = asRecord(value);
  const topic = stringField(record, "topic");
  if (!topic) {
    return null;
  }

  return {
    topic,
    stageLevel: stringField(record, "stageLevel") ?? "intuition",
    score: normalizedScore(record?.score),
    passed: booleanField(record, "passed") ?? false,
    feedback: stringField(record, "feedback") ?? "",
    masteryEvidence: stringArrayField(record, "masteryEvidence"),
    gaps: stringArrayField(record, "gaps"),
    submittedAt: stringField(record, "submittedAt") ?? "",
  };
}

function normalizeCurriculumCoverage(value: unknown): CurriculumCoverage | null {
  const record = asRecord(value);
  const learningArea = stringField(record, "learningArea");
  if (!learningArea) {
    return null;
  }

  return {
    learningArea,
    strand: stringField(record, "strand") ?? "",
    yearLevel: stringField(record, "yearLevel") ?? "",
    referenceId: stringField(record, "referenceId") ?? learningArea,
    referenceLabel: stringField(record, "referenceLabel") ?? learningArea,
    sourceUrl: stringField(record, "sourceUrl") ?? "",
    status: stringField(record, "status") ?? "not_evidenced",
    evidenceTopics: stringArrayField(record, "evidenceTopics"),
    evidenceCount: Math.max(
      0,
      Math.floor(numberField(record, "evidenceCount") ?? 0),
    ),
    averageScore: normalizedScore(record?.averageScore),
    parentNote: stringField(record, "parentNote") ?? "",
  };
}

function normalizeSource(value: unknown): CurriculumSource | null {
  const record = asRecord(value);
  const label = stringField(record, "label");
  const url = stringField(record, "url");
  return label && url ? { label, url } : null;
}

function normalizedScore(value: unknown): number {
  const score = typeof value === "number" && Number.isFinite(value) ? value : 0;
  return Math.max(0, Math.min(1, score));
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object"
    ? (value as Record<string, unknown>)
    : null;
}

function arrayField(
  record: Record<string, unknown> | null,
  key: string,
): unknown[] {
  const value = record?.[key];
  return Array.isArray(value) ? value : [];
}

function stringField(
  record: Record<string, unknown> | null,
  key: string,
): string | undefined {
  const value = record?.[key];
  return typeof value === "string" ? value : undefined;
}

function numberField(
  record: Record<string, unknown> | null,
  key: string,
): number | undefined {
  const value = record?.[key];
  return typeof value === "number" && Number.isFinite(value)
    ? value
    : undefined;
}

function booleanField(
  record: Record<string, unknown> | null,
  key: string,
): boolean | undefined {
  const value = record?.[key];
  return typeof value === "boolean" ? value : undefined;
}

function stringArrayField(
  record: Record<string, unknown> | null,
  key: string,
): string[] {
  const value = record?.[key];
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

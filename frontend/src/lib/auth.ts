export type AuthSession = {
  token: string;
  type: string;
};

export type AuthenticatedStudent = {
  studentId: string;
  displayName: string;
  age?: number | null;
  ageBand: string;
  biography?: string | null;
  interests: string[];
  preferredExplanationStyle: string;
  levelContext: string;
  memories?: unknown[];
  progress?: unknown[];
  suggestedTopics: string[];
  xpTotal: number;
};

export type AuthPayload = {
  error?: string;
  session?: AuthSession | null;
  student?: AuthenticatedStudent | null;
};

export type StoredAuth = {
  session: AuthSession | null;
  student: AuthenticatedStudent;
};

export const apiBaseUrl =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:4000";

const authStorageKey = "primerlab-auth";

export function authHeaders(session: AuthSession | null): Record<string, string> {
  if (!session?.token) {
    return {};
  }

  return {
    Authorization: `Bearer ${session.token}`,
  };
}

export function readStoredAuth(): StoredAuth | null {
  if (typeof window === "undefined") {
    return null;
  }

  const raw = window.localStorage.getItem(authStorageKey);
  if (!raw) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as StoredAuth;
    const student = normalizeAuthenticatedStudent(parsed?.student);
    return student ? { session: parsed.session ?? null, student } : null;
  } catch {
    window.localStorage.removeItem(authStorageKey);
    return null;
  }
}

export function storeAuth(auth: StoredAuth) {
  const student = normalizeAuthenticatedStudent(auth.student);
  if (!student) {
    return;
  }

  window.localStorage.setItem(
    authStorageKey,
    JSON.stringify({ ...auth, student }),
  );
}

export function clearStoredAuth() {
  window.localStorage.removeItem(authStorageKey);
}

export function splitInterests(value: string): string[] {
  return value
    .split(",")
    .map((interest) => interest.trim())
    .filter(Boolean);
}

export function normalizeAuthenticatedStudent(
  value: unknown,
): AuthenticatedStudent | null {
  const record = asRecord(value);
  const studentId = stringField(record, "studentId");
  if (!studentId) {
    return null;
  }

  return {
    studentId,
    displayName: stringField(record, "displayName") ?? studentId,
    age: numberField(record, "age") ?? null,
    ageBand: stringField(record, "ageBand") ?? "",
    biography: stringField(record, "biography") ?? null,
    interests: stringArrayField(record, "interests"),
    preferredExplanationStyle:
      stringField(record, "preferredExplanationStyle") ?? "",
    levelContext: stringField(record, "levelContext") ?? "",
    memories: arrayField(record, "memories"),
    progress: arrayField(record, "progress"),
    suggestedTopics: stringArrayField(record, "suggestedTopics"),
    xpTotal: Math.max(0, Math.floor(numberField(record, "xpTotal") ?? 0)),
  };
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

function arrayField(
  record: Record<string, unknown> | null,
  key: string,
): unknown[] | undefined {
  const value = record?.[key];
  return Array.isArray(value) ? value : undefined;
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

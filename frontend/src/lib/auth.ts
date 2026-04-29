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
    return parsed?.student?.studentId ? parsed : null;
  } catch {
    window.localStorage.removeItem(authStorageKey);
    return null;
  }
}

export function storeAuth(auth: StoredAuth) {
  window.localStorage.setItem(authStorageKey, JSON.stringify(auth));
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

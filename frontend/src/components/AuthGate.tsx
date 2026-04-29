"use client";

import {
  BookOpen,
  CalendarDays,
  KeyRound,
  LogIn,
  Sparkles,
  UserRound,
} from "lucide-react";
import { type ComponentType, type FormEvent, type ReactNode, useState } from "react";
import {
  type AuthPayload,
  apiBaseUrl,
  splitInterests,
} from "@/lib/auth";

type AuthGateProps = {
  onAuthenticated: (payload: AuthPayload) => void;
};

type AuthMode = "signup" | "login";

export function AuthGate({ onAuthenticated }: AuthGateProps) {
  const [mode, setMode] = useState<AuthMode>("signup");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [activationCode, setActivationCode] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [age, setAge] = useState("");
  const [biography, setBiography] = useState("");
  const [interests, setInterests] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const isSignup = mode === "signup";

  async function submitAuth(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      const response = await fetch(
        `${apiBaseUrl}/api/auth/${isSignup ? "register" : "login"}`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(
            isSignup
              ? {
                  username,
                  password,
                  activationCode,
                  displayName,
                  age: Number(age),
                  biography,
                  interests: splitInterests(interests),
                }
              : {
                  username,
                  password,
                },
          ),
        },
      );
      const payload = (await response.json()) as AuthPayload;
      if (payload.error || !payload.student) {
        setError(payload.error ?? "Authentication failed.");
        return;
      }

      onAuthenticated(payload);
    } catch (caught) {
      setError(`Could not reach backend: ${String(caught)}`);
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <main className="relative min-h-[100svh] overflow-hidden bg-[#111515] text-stone-100">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_18%_14%,rgba(76,139,141,0.34),transparent_34%),radial-gradient(circle_at_84%_22%,rgba(215,170,92,0.2),transparent_30%),linear-gradient(135deg,#101818_0%,#182321_56%,#0d1112_100%)]" />
      <section className="relative z-10 mx-auto flex min-h-[100svh] w-full max-w-[430px] flex-col justify-center px-5 py-8">
        <div className="mb-7 text-center">
          <div className="inline-flex items-center gap-2 text-xs uppercase text-cyan-50/78">
            <BookOpen className="h-3.5 w-3.5" />
            Primer
          </div>
          <h1 className="mt-4 text-3xl font-semibold leading-tight text-stone-50">
            {isSignup ? "Create student profile" : "Welcome back"}
          </h1>
        </div>

        <div className="flex rounded-[8px] border border-cyan-100/12 bg-black/20 p-1">
          <button
            type="button"
            onClick={() => setMode("signup")}
            className={`h-10 flex-1 rounded-[6px] text-sm font-semibold transition ${
              isSignup
                ? "bg-[#d8b86a] text-[#17201d]"
                : "text-cyan-50/72 hover:text-cyan-50"
            }`}
          >
            Sign up
          </button>
          <button
            type="button"
            onClick={() => setMode("login")}
            className={`h-10 flex-1 rounded-[6px] text-sm font-semibold transition ${
              !isSignup
                ? "bg-[#d8b86a] text-[#17201d]"
                : "text-cyan-50/72 hover:text-cyan-50"
            }`}
          >
            Sign in
          </button>
        </div>

        <form className="mt-5 grid gap-4" onSubmit={submitAuth}>
          <AuthField icon={UserRound} label="Username" htmlFor="username">
            <input
              id="username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              minLength={3}
              required
              className="auth-input"
              autoComplete="username"
            />
          </AuthField>

          <AuthField icon={KeyRound} label="Password" htmlFor="password">
            <input
              id="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              minLength={8}
              required
              type="password"
              className="auth-input"
              autoComplete={isSignup ? "new-password" : "current-password"}
            />
          </AuthField>

          {isSignup ? (
            <>
              <AuthField
                icon={KeyRound}
                label="Activation code"
                htmlFor="activation-code"
              >
                <input
                  id="activation-code"
                  value={activationCode}
                  onChange={(event) => setActivationCode(event.target.value)}
                  required
                  type="password"
                  className="auth-input"
                  autoComplete="one-time-code"
                />
              </AuthField>

              <AuthField
                icon={Sparkles}
                label="Display name"
                htmlFor="display-name"
              >
                <input
                  id="display-name"
                  value={displayName}
                  onChange={(event) => setDisplayName(event.target.value)}
                  required
                  className="auth-input"
                  autoComplete="given-name"
                />
              </AuthField>

              <AuthField icon={CalendarDays} label="Age" htmlFor="age">
                <input
                  id="age"
                  value={age}
                  onChange={(event) => setAge(event.target.value)}
                  min={5}
                  max={18}
                  required
                  type="number"
                  className="auth-input"
                  inputMode="numeric"
                />
              </AuthField>

              <AuthField
                icon={Sparkles}
                label="Biography"
                htmlFor="biography"
              >
                <textarea
                  id="biography"
                  value={biography}
                  onChange={(event) => setBiography(event.target.value)}
                  required
                  className="auth-textarea"
                  placeholder="Loves tide pools, builds paper machines, asks why storms happen"
                  rows={4}
                />
              </AuthField>

              <AuthField
                icon={Sparkles}
                label="Interests"
                htmlFor="interests"
              >
                <input
                  id="interests"
                  value={interests}
                  onChange={(event) => setInterests(event.target.value)}
                  required
                  className="auth-input"
                  placeholder="marine biology, drawing, puzzles"
                />
              </AuthField>
            </>
          ) : null}

          {error ? (
            <p className="rounded-[8px] border border-red-300/25 bg-red-950/28 px-3 py-2 text-sm leading-6 text-red-100">
              {error}
            </p>
          ) : null}

          <button
            type="submit"
            disabled={isSubmitting}
            className="mt-1 inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-5 text-sm font-semibold text-[#17201d] transition hover:bg-[#e5c879] disabled:cursor-not-allowed disabled:opacity-60"
          >
            <LogIn className="h-4 w-4" />
            {isSubmitting
              ? "Checking..."
              : isSignup
                ? "Create student profile"
                : "Sign in"}
          </button>
        </form>
      </section>
    </main>
  );
}

function AuthField({
  children,
  htmlFor,
  icon: Icon,
  label,
}: {
  children: ReactNode;
  htmlFor: string;
  icon: ComponentType<{ className?: string }>;
  label: string;
}) {
  return (
    <label htmlFor={htmlFor} className="block">
      <span className="flex items-center gap-2 text-xs uppercase text-cyan-50/62">
        <Icon className="h-3.5 w-3.5" />
        {label}
      </span>
      <span className="mt-2 block">{children}</span>
    </label>
  );
}

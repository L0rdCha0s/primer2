"use client";

import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
  type NodeMouseHandler,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import {
  Atom,
  Bolt,
  BookOpen,
  Brain,
  Check,
  ChevronLeft,
  ChevronRight,
  Cloud,
  Compass,
  Cog,
  Feather,
  Leaf,
  Lock,
  LogOut,
  Map as MapIcon,
  Mic,
  Play,
  Search,
  Sparkles,
  Square,
  Unlock,
  Volume2,
  Waves,
  X,
} from "lucide-react";
import { AuthGate } from "@/components/AuthGate";
import HTMLFlipBook from "react-pageflip";
import {
  type InfographicSpec,
  type Stage,
  type StagegateResult,
  type StudentMemory,
  lightningInfographic,
  memories,
  seededStagegateResult,
  stages,
  student,
  tutorScene,
  unlockedMemory,
} from "@/lib/demo-data";
import {
  type AuthPayload,
  type AuthSession,
  type AuthenticatedStudent,
  authHeaders,
  apiBaseUrl,
  clearStoredAuth,
  readStoredAuth,
  storeAuth,
} from "@/lib/auth";
import {
  type ComponentType,
  type CSSProperties,
  type ReactNode,
  forwardRef,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

type FlipCorner = "top" | "bottom";

type PageFlipApi = {
  flipNext: (corner?: FlipCorner) => void;
  flipPrev: (corner?: FlipCorner) => void;
  flip: (page: number, corner?: FlipCorner) => void;
  getPageCount: () => number;
  getCurrentPageIndex: () => number;
};

type FlipBookRef = {
  pageFlip: () => PageFlipApi | undefined;
};

type PageFlipEvent<T> = {
  data: T;
};

type PrimerLesson = {
  topic: string;
  stageLevel: "intuition" | "mechanism" | "transfer";
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

type InfographicArtifact = {
  aiMode?: string;
  generated?: boolean;
  imageDataUrl?: string | null;
  imageUrl?: string | null;
  message?: string;
  prompt?: string;
  model?: string;
};

type NarrationPayload = {
  aiMode?: string;
  generated?: boolean;
  audioDataUrl?: string | null;
  contentType?: string;
  error?: string;
  message?: string;
  model?: string;
  voice?: string;
};

type BackendMemory = {
  assertionId?: string;
  memory_type?: StudentMemory["type"];
  memoryType?: StudentMemory["type"];
  content?: string;
  confidence?: number;
  tags?: string[];
  subject?: string;
  predicate?: string;
  validFrom?: string;
  validTo?: string;
  knownFrom?: string;
  knownTo?: string;
  source?: string;
};

type MemoryGraphNodeRecord = {
  id: string;
  nodeType: string;
  kind: string;
  label: string;
  summary?: string | null;
  expanded: boolean;
  factCount: number;
};

type MemoryGraphEdgeRecord = {
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

type StudentMemoryGraph = {
  studentId: string;
  rootNodeId: string;
  selectedNodeId: string;
  nodes: MemoryGraphNodeRecord[];
  edges: MemoryGraphEdgeRecord[];
  validAsOf: string;
  knownAsOf: string;
};

type MemoryNodeData = Record<string, unknown> & {
  label: string;
  kind: string;
  nodeType: string;
  factCount: number;
  expanded: boolean;
  summary?: string | null;
};

type MemoryEdgeData = Record<string, unknown> & {
  label: string;
  content: string;
  memoryType: string;
  confidence: number;
};

type MemoryFlowNode = Node<MemoryNodeData>;
type MemoryFlowEdge = Edge<MemoryEdgeData>;

type BookPageProps = {
  children: ReactNode;
  density?: "hard" | "soft";
  pageNumber?: number;
  tone?: "cover" | "paper" | "deep";
};

const BookPage = forwardRef<HTMLDivElement, BookPageProps>(
  ({ children, density = "soft", pageNumber, tone = "paper" }, ref) => {
    return (
      <section
        ref={ref}
        data-density={density}
        className={`primer-page primer-page-${tone}`}
      >
        <div className="primer-page-grain" />
        <div className="relative z-10 flex h-full flex-col overflow-hidden px-7 py-7 sm:px-8 sm:py-8">
          {children}
          {typeof pageNumber === "number" ? (
            <div className="mt-auto flex items-center justify-between border-t border-stone-300/70 pt-3 text-[11px] uppercase text-stone-500">
              <span>The Clockwork Reef</span>
              <span>{pageNumber}</span>
            </div>
          ) : null}
        </div>
      </section>
    );
  },
);

BookPage.displayName = "BookPage";

const iconMap: Record<
  InfographicSpec["panels"][number]["icon"],
  ComponentType<{ className?: string; strokeWidth?: number }>
> = {
  atom: Atom,
  bolt: Bolt,
  brain: Brain,
  cloud: Cloud,
  gear: Cog,
  leaf: Leaf,
  magnifier: Search,
  map: MapIcon,
  spark: Sparkles,
  water: Waves,
};

const pageTurnStyle: CSSProperties = {};

const initialLesson: PrimerLesson = {
  topic: "lightning",
  stageLevel: "intuition",
  communicationStyle: "Visual, story-first, ocean-current analogies.",
  storyScene: tutorScene.storyScene,
  plainExplanation: tutorScene.plainExplanation,
  analogy: tutorScene.analogy,
  checkForUnderstanding: tutorScene.check,
  suggestedTopics: [
    "coral reef ecosystems",
    "fractions through music",
    "photosynthesis",
    "magnetism",
  ],
  stagegatePrompt:
    "Explain the most important idea in your own words, then connect it to one new example.",
  infographicPrompt:
    "Create an age-appropriate Clockwork Reef infographic explaining lightning.",
  keyTerms: lightningInfographic.keyTerms,
};

async function fetchStudentMemoryGraph(
  learner: AuthenticatedStudent,
  session: AuthSession | null,
  nodeId?: string,
): Promise<StudentMemoryGraph | null> {
  const response = await fetch(`${apiBaseUrl}/api/memory/graph`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeaders(session) },
    body: JSON.stringify({
      studentId: learner.studentId,
      nodeId,
      maxEdges: 36,
    }),
  });
  const payload = await response.json();
  if (payload.error || !payload.graph) {
    return null;
  }

  return normalizeMemoryGraph(payload.graph);
}

export function PrimerBook() {
  const bookRef = useRef<FlipBookRef | null>(null);
  const narrationAudioRef = useRef<HTMLAudioElement | null>(null);
  const [authChecked, setAuthChecked] = useState(false);
  const [authenticatedStudent, setAuthenticatedStudent] =
    useState<AuthenticatedStudent | null>(null);
  const [session, setSession] = useState<AuthSession | null>(null);
  const [currentPage, setCurrentPage] = useState(0);
  const [topic, setTopic] = useState("lightning");
  const [lesson, setLesson] = useState<PrimerLesson>(initialLesson);
  const [remoteMemories, setRemoteMemories] = useState<StudentMemory[] | null>(
    null,
  );
  const [memoryGraph, setMemoryGraph] = useState<StudentMemoryGraph | null>(
    null,
  );
  const [isMemoryOpen, setIsMemoryOpen] = useState(false);
  const [memoryGraphStatus, setMemoryGraphStatus] = useState(
    "Memory graph not loaded.",
  );
  const [selectedMemoryNodeId, setSelectedMemoryNodeId] = useState<
    string | null
  >(null);
  const [hasAsked, setHasAsked] = useState(false);
  const [hasGeneratedInfographic, setHasGeneratedInfographic] = useState(false);
  const [hasPassedStagegate, setHasPassedStagegate] = useState(false);
  const [lessonStatus, setLessonStatus] = useState(
    "Choose a topic and ask the Primer.",
  );
  const [infographicStatus, setInfographicStatus] = useState(
    "No generated infographic yet.",
  );
  const [infographicArtifact, setInfographicArtifact] =
    useState<InfographicArtifact | null>(null);
  const [stagegateResult, setStagegateResult] =
    useState<StagegateResult>(seededStagegateResult);
  const [answer, setAnswer] = useState(
    "I think the important idea is that a hidden cause builds up, then something visible happens when it crosses a limit.",
  );
  const [isNarrating, setIsNarrating] = useState(false);
  const [isNarrationLoading, setIsNarrationLoading] = useState(false);
  const [narrationStatus, setNarrationStatus] = useState(
    "OpenAI narration is ready.",
  );
  const learnerName = authenticatedStudent?.displayName ?? student.displayName;

  useEffect(() => {
    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled) {
        return;
      }

      const stored = readStoredAuth();
      if (stored?.student) {
        setAuthenticatedStudent(stored.student);
        setSession(stored.session);
        setRemoteMemories(normalizeMemories(stored.student.memories));
        if (stored.student.suggestedTopics.length > 0) {
          setLesson((currentLesson) => ({
            ...currentLesson,
            suggestedTopics: stored.student.suggestedTopics,
          }));
        }
        if (stored.student.interests.length > 0) {
          setTopic(stored.student.interests[0]);
        }
      }
      setAuthChecked(true);
    });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    return () => {
      const audio = narrationAudioRef.current;
      if (audio) {
        audio.pause();
        audio.src = "";
      }
      narrationAudioRef.current = null;
    };
  }, []);

  async function loadMemoryGraph(nodeId?: string) {
    if (!authenticatedStudent) {
      return;
    }

    const isInitialLoad = !nodeId;
    setMemoryGraphStatus(
      isInitialLoad ? "Loading memory graph..." : "Loading connected memories...",
    );

    try {
      const graph = await fetchStudentMemoryGraph(
        authenticatedStudent,
        session,
        nodeId,
      );

      if (!graph) {
        setMemoryGraphStatus("No graph memory is available yet.");
        return;
      }

      setMemoryGraph((currentGraph) =>
        isInitialLoad ? graph : mergeMemoryGraph(currentGraph, graph),
      );
      setSelectedMemoryNodeId(graph.selectedNodeId);
      setMemoryGraphStatus(
        isInitialLoad
          ? "Relational bitemporal graph loaded."
          : "Connected memory nodes loaded.",
      );
    } catch (error) {
      setMemoryGraphStatus(`Memory graph unavailable: ${String(error)}`);
    }
  }

  function openMemoryGraph() {
    setIsMemoryOpen(true);
    if (!memoryGraph) {
      void loadMemoryGraph();
    }
  }

  function walkMemoryNode(nodeId: string) {
    setSelectedMemoryNodeId(nodeId);

    const node = memoryGraph?.nodes.find((candidate) => candidate.id === nodeId);
    if (node?.expanded) {
      setMemoryGraphStatus("Connected memory nodes already loaded.");
      return;
    }

    void loadMemoryGraph(nodeId);
  }

  const visibleMemories = useMemo<StudentMemory[]>(() => {
    const baseMemories = remoteMemories ?? memories;
    return hasPassedStagegate
      ? [
          ...baseMemories,
          {
            ...unlockedMemory,
            content: `${learnerName} made progress on ${lesson.topic} at the ${lesson.stageLevel} level.`,
          },
        ]
      : baseMemories;
  }, [
    hasPassedStagegate,
    learnerName,
    lesson.stageLevel,
    lesson.topic,
    remoteMemories,
  ]);

  const visibleStages = useMemo<Stage[]>(() => {
    if (!hasPassedStagegate) {
      return stages;
    }

    return stages.map((stage) => {
      if (stage.level === "intuition") {
        return { ...stage, status: "passed" };
      }

      if (stage.level === "mechanism") {
        return { ...stage, status: "available" };
      }

      return stage;
    });
  }, [hasPassedStagegate]);

  const pageCount = 10;

  function flipNext() {
    bookRef.current?.pageFlip()?.flipNext("bottom");
  }

  function flipPrev() {
    bookRef.current?.pageFlip()?.flipPrev("bottom");
  }

  function goToPage(page: number) {
    bookRef.current?.pageFlip()?.flip(page, "bottom");
  }

  function clearNarrationAudio() {
    const audio = narrationAudioRef.current;
    if (!audio) {
      return;
    }

    audio.pause();
    audio.src = "";
    narrationAudioRef.current = null;
  }

  function stopNarration(message = "Narration stopped.") {
    clearNarrationAudio();
    setIsNarrating(false);
    setIsNarrationLoading(false);
    setNarrationStatus(message);
  }

  async function playNarration() {
    if (isNarrationLoading) {
      return;
    }

    if (isNarrating || narrationAudioRef.current) {
      stopNarration();
      return;
    }

    if (!authenticatedStudent) {
      return;
    }

    const narrationText = [
      lesson.storyScene,
      lesson.plainExplanation,
      lesson.analogy,
    ]
      .join("\n\n")
      .trim();

    if (!narrationText) {
      setNarrationStatus("There is no lesson text to narrate yet.");
      return;
    }

    setIsNarrationLoading(true);
    setNarrationStatus("Generating fable narration with OpenAI TTS...");

    try {
      const response = await fetch(`${apiBaseUrl}/api/narration/speech`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders(session) },
        body: JSON.stringify({
          studentId: authenticatedStudent.studentId,
          topic: lesson.topic,
          text: narrationText,
          instructions:
            "Use the fable voice as a calm, precise book narrator for a curious learner.",
        }),
      });
      const payload = await response.json();
      const narration = payload.narration as NarrationPayload | undefined;

      if (!narration?.generated || !narration.audioDataUrl) {
        setNarrationStatus(
          narration?.message ??
            narration?.error ??
            "OpenAI TTS narration was unavailable.",
        );
        return;
      }

      const audio = new Audio(narration.audioDataUrl);
      narrationAudioRef.current = audio;
      audio.onended = () => {
        narrationAudioRef.current = null;
        setIsNarrating(false);
        setNarrationStatus(
          `Narrated with ${narration.voice ?? "fable"} via ${
            narration.model ?? "OpenAI TTS"
          }.`,
        );
      };
      audio.onerror = () => {
        narrationAudioRef.current = null;
        setIsNarrating(false);
        setNarrationStatus("Could not play the generated narration.");
      };

      setIsNarrating(true);
      setNarrationStatus(
        `Playing AI-generated narration with ${
          narration.voice ?? "fable"
        } voice.`,
      );
      await audio.play();
    } catch (error) {
      clearNarrationAudio();
      setIsNarrating(false);
      setNarrationStatus(`Could not generate narration: ${String(error)}`);
    } finally {
      setIsNarrationLoading(false);
    }
  }

  async function startTopic(nextTopic = topic) {
    if (!authenticatedStudent) {
      return;
    }

    const cleanTopic = nextTopic.trim();
    if (!cleanTopic) {
      return;
    }

    stopNarration("OpenAI narration is ready.");
    setTopic(cleanTopic);
    setLessonStatus("Asking OpenAI Responses to guide this path...");
    setHasAsked(true);
    setHasGeneratedInfographic(false);
    setInfographicArtifact(null);
    setHasPassedStagegate(false);

    try {
      const response = await fetch(`${apiBaseUrl}/api/lesson/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders(session) },
        body: JSON.stringify({
          studentId: authenticatedStudent.studentId,
          topic: cleanTopic,
          question: `I want to explore ${cleanTopic}. Guide me at my current level.`,
        }),
      });
      const payload = await response.json();
      if (payload.error || !payload.lesson) {
        setRemoteMemories(normalizeMemories(payload.student?.memories));
        setLessonStatus(
          payload.error ?? "The Primer could not generate this lesson yet.",
        );
        if (isMemoryOpen) {
          void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
        }
        return;
      }

      setLesson(normalizeLesson(payload.lesson, cleanTopic));
      setRemoteMemories(normalizeMemories(payload.student?.memories));
      setLessonStatus(
        payload.lesson?.aiMode === "openai_responses"
          ? "Guided by OpenAI Responses."
          : "Guided by the Primer.",
      );
      if (isMemoryOpen) {
        void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
      }
      goToPage(4);
    } catch (error) {
      setLessonStatus(`Could not reach backend: ${String(error)}`);
    }
  }

  async function generateInfographic() {
    if (!authenticatedStudent) {
      return;
    }

    setInfographicStatus("Calling gpt-image-2 for an infographic...");

    try {
      const response = await fetch(`${apiBaseUrl}/api/artifact/infographic`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders(session) },
        body: JSON.stringify({
          studentId: authenticatedStudent.studentId,
          topic: lesson.topic,
          lessonSummary: lesson.plainExplanation,
          infographicPrompt: lesson.infographicPrompt,
          size: "1024x1024",
        }),
      });
      const payload = await response.json();
      setInfographicArtifact(payload.artifact);
      setHasGeneratedInfographic(Boolean(payload.artifact?.generated));
      setInfographicStatus(
        payload.artifact?.generated
          ? `Generated with ${payload.artifact.model ?? "gpt-image-2"}.`
          : (payload.artifact?.message ??
              "Set OPENAI_API_KEY in backend/.env to generate an image."),
      );
    } catch (error) {
      setInfographicStatus(`Could not reach backend: ${String(error)}`);
    }
  }

  async function submitStagegate() {
    if (!authenticatedStudent) {
      return;
    }

    try {
      const response = await fetch(`${apiBaseUrl}/api/tutor/stagegate`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders(session) },
        body: JSON.stringify({
          studentId: authenticatedStudent.studentId,
          topic: lesson.topic,
          answer,
          stageLevel: lesson.stageLevel,
        }),
      });
      const payload = await response.json();
      if (payload.error || !payload.result) {
        setRemoteMemories(normalizeMemories(payload.student?.memories));
        setHasPassedStagegate(false);
        setStagegateResult({
          ...seededStagegateResult,
          passed: false,
          score: 0,
          rubric: {
            accuracy: 0,
            causalReasoning: 0,
            vocabulary: 0,
            transfer: 0,
          },
          masteryEvidence: [],
          gaps: ["The backend stagegate assessor was unavailable."],
          feedbackToStudent:
            payload.error ?? "The Primer could not grade this answer yet.",
        });
        if (isMemoryOpen) {
          void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
        }
        window.setTimeout(flipNext, 250);
        return;
      }

      setStagegateResult(normalizeStagegateResult(payload.result));
      setRemoteMemories(normalizeMemories(payload.student?.memories));
      setHasPassedStagegate(Boolean(payload.result?.passed));
      if (isMemoryOpen) {
        void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
      }
    } catch (error) {
      setHasPassedStagegate(false);
      setStagegateResult({
        ...seededStagegateResult,
        passed: false,
        score: 0,
        rubric: {
          accuracy: 0,
          causalReasoning: 0,
          vocabulary: 0,
          transfer: 0,
        },
        masteryEvidence: [],
        gaps: ["The backend stagegate assessor was unavailable."],
        feedbackToStudent: `Could not reach backend: ${String(error)}`,
      });
    }

    window.setTimeout(flipNext, 250);
  }

  function handleAuthenticated(payload: AuthPayload) {
    if (!payload.student) {
      return;
    }

    const studentProfile = payload.student;
    const nextSession = payload.session ?? null;
    setAuthenticatedStudent(studentProfile);
    setSession(nextSession);
    setRemoteMemories(normalizeMemories(studentProfile.memories));
    if (studentProfile.suggestedTopics.length > 0) {
      setLesson((currentLesson) => ({
        ...currentLesson,
        suggestedTopics: studentProfile.suggestedTopics,
      }));
    }
    if (studentProfile.interests.length > 0) {
      setTopic(studentProfile.interests[0]);
    }
    storeAuth({ student: studentProfile, session: nextSession });
  }

  function handleLogout() {
    clearStoredAuth();
    stopNarration("OpenAI narration is ready.");
    setAuthenticatedStudent(null);
    setSession(null);
    setRemoteMemories(null);
    setMemoryGraph(null);
    setIsMemoryOpen(false);
    setMemoryGraphStatus("Memory graph not loaded.");
    setSelectedMemoryNodeId(null);
    setCurrentPage(0);
    setTopic("lightning");
    setLesson(initialLesson);
    setHasAsked(false);
    setHasGeneratedInfographic(false);
    setHasPassedStagegate(false);
    setInfographicArtifact(null);
    setLessonStatus("Choose a topic and ask the Primer.");
    setInfographicStatus("No generated infographic yet.");
  }

  if (!authChecked) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-[#111515] text-stone-100">
        <div className="inline-flex items-center gap-2 text-sm uppercase text-cyan-50/70">
          <BookOpen className="h-4 w-4" />
          Opening PrimerLab
        </div>
      </main>
    );
  }

  if (!authenticatedStudent) {
    return <AuthGate onAuthenticated={handleAuthenticated} />;
  }

  const learner = authenticatedStudent;

  return (
    <main className="relative min-h-[100svh] overflow-hidden bg-[#111515] text-stone-100">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_10%,rgba(76,139,141,0.32),transparent_34%),radial-gradient(circle_at_82%_28%,rgba(215,170,92,0.18),transparent_28%),linear-gradient(135deg,#101818_0%,#182321_54%,#0d1112_100%)]" />
      <div className="absolute inset-x-0 top-0 h-px bg-cyan-100/20" />

      <section className="book-stage relative z-10 flex min-h-[100svh] w-full flex-col items-center justify-center gap-3 px-2 pb-3 pt-14 sm:px-5 sm:pb-5 sm:pt-16">
        <div className="absolute left-3 top-3 z-30 hidden items-center gap-2 rounded-full border border-cyan-100/15 bg-black/24 px-3 py-2 text-sm text-cyan-50 shadow-2xl shadow-black/20 sm:flex">
          <BookOpen className="h-4 w-4" />
          <span>{learner.displayName}&apos;s Primer</span>
        </div>
        <div className="absolute right-3 top-3 z-30 flex items-center gap-2">
          <button
            type="button"
            onClick={openMemoryGraph}
            className="inline-flex h-10 items-center gap-2 rounded-full border border-cyan-100/15 bg-black/28 px-3 text-sm text-cyan-50/78 shadow-2xl shadow-black/20 transition hover:text-cyan-50"
          >
            <Brain className="h-4 w-4" />
            Memory
          </button>
          <button
            type="button"
            onClick={handleLogout}
            className="inline-flex h-10 items-center gap-2 rounded-full border border-cyan-100/15 bg-black/28 px-3 text-sm text-cyan-50/78 shadow-2xl shadow-black/20 transition hover:text-cyan-50"
          >
            <LogOut className="h-4 w-4" />
            Sign out
          </button>
        </div>

        <div className="primer-book-shell relative">
          <div className="absolute inset-x-8 top-1/2 h-10 -translate-y-1/2 rounded-full bg-black/35 blur-3xl" />
          <HTMLFlipBook
            ref={bookRef}
            className="primer-flipbook mx-auto"
            style={pageTurnStyle}
            startPage={0}
            size="stretch"
            width={430}
            height={620}
            minWidth={300}
            maxWidth={780}
            minHeight={430}
            maxHeight={1120}
            drawShadow
            flippingTime={720}
            usePortrait
            startZIndex={20}
            autoSize
            maxShadowOpacity={0.28}
            showCover
            mobileScrollSupport
            clickEventForward
            useMouseEvents
            swipeDistance={28}
            showPageCorners
            disableFlipByClick={false}
            onFlip={(event: PageFlipEvent<number>) =>
              setCurrentPage(Number(event.data))
            }
          >
                <BookPage density="hard" tone="cover">
                  <CoverPage learner={learner} />
                </BookPage>

                <BookPage pageNumber={1}>
                  <WelcomePage learner={learner} memories={visibleMemories} />
                </BookPage>

                <BookPage pageNumber={2}>
                  <StageMapPage stages={visibleStages} />
                </BookPage>

                <BookPage pageNumber={3}>
                  <AskPage
                    hasAsked={hasAsked}
                    lessonStatus={lessonStatus}
                    suggestedTopics={lesson.suggestedTopics}
                    topic={topic}
                    onAsk={() => void startTopic()}
                    onChooseTopic={(nextTopic) => void startTopic(nextTopic)}
                    onTopicChange={setTopic}
                  />
                </BookPage>

                <BookPage pageNumber={4}>
                  <StoryPage lesson={lesson} />
                </BookPage>

                <BookPage pageNumber={5}>
                  <InfographicPage
                    artifact={infographicArtifact}
                    infographicStatus={infographicStatus}
                    infographic={lightningInfographic}
                    lesson={lesson}
                    hasGeneratedInfographic={hasGeneratedInfographic}
                    onGenerate={() => void generateInfographic()}
                  />
                </BookPage>

                <BookPage pageNumber={6}>
                  <VoiceoverPage
                    lesson={lesson}
                    isNarrating={isNarrating}
                    isNarrationLoading={isNarrationLoading}
                    narrationStatus={narrationStatus}
                    onPlayNarration={() => void playNarration()}
                  />
                </BookPage>

                <BookPage pageNumber={7}>
                  <FollowUpPage lesson={lesson} />
                </BookPage>

                <BookPage pageNumber={8}>
                  <StagegatePage
                    answer={answer}
                    hasPassed={hasPassedStagegate}
                    lesson={lesson}
                    onAnswerChange={setAnswer}
                    onSubmit={submitStagegate}
                  />
                </BookPage>

                <BookPage density="hard" tone="deep">
                  <UnlockPage
                    result={stagegateResult}
                    hasPassed={hasPassedStagegate}
                  />
                </BookPage>
          </HTMLFlipBook>

          <div className="relative z-20 mt-3 flex flex-wrap items-center justify-center gap-2 sm:mt-4 sm:gap-3">
                <button
                  type="button"
                  onClick={flipPrev}
                  className="inline-flex h-11 items-center gap-2 rounded-full border border-cyan-100/15 bg-cyan-50/10 px-4 text-sm text-cyan-50 transition hover:bg-cyan-50/16"
                >
                  <ChevronLeft className="h-4 w-4" />
                  Previous
                </button>
                <div className="min-w-[160px] rounded-full border border-stone-200/10 bg-black/22 px-4 py-2 text-center text-sm text-stone-200">
                  Page {Math.min(currentPage + 1, pageCount)} of {pageCount}
                </div>
                <button
                  type="button"
                  onClick={flipNext}
                  className="inline-flex h-11 items-center gap-2 rounded-full bg-[#d8b86a] px-4 text-sm font-semibold text-[#19221f] transition hover:bg-[#e4c77a]"
                >
                  Next
                  <ChevronRight className="h-4 w-4" />
                </button>
          </div>
        </div>
      </section>
      {isMemoryOpen ? (
        <MemoryGraphDialog
          graph={memoryGraph}
          status={memoryGraphStatus}
          selectedNodeId={selectedMemoryNodeId}
          onClose={() => setIsMemoryOpen(false)}
          onRefresh={() => void loadMemoryGraph(selectedMemoryNodeId ?? undefined)}
          onWalkNode={walkMemoryNode}
        />
      ) : null}
    </main>
  );
}

function CoverPage({ learner }: { learner: AuthenticatedStudent }) {
  return (
    <div className="flex h-full flex-col justify-between text-stone-50">
      <div>
        <div className="inline-flex items-center gap-2 border border-cyan-100/25 px-3 py-1 text-xs uppercase text-cyan-50/80">
          <Feather className="h-3.5 w-3.5" />
          Adaptive story tutor
        </div>
        <h2 className="mt-8 max-w-[12ch] text-5xl font-semibold leading-[1.02] sm:text-6xl">
          The Clockwork Reef
        </h2>
        <p className="mt-5 max-w-xs text-base leading-7 text-cyan-50/78">
          A living lesson book for {learner.displayName}, where interests steer
          examples, gates, and the next thing to explore.
        </p>
      </div>

      <div className="relative h-48 overflow-hidden rounded-[8px] border border-cyan-100/20 bg-cyan-50/10">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_45%_45%,rgba(216,184,106,0.38),transparent_22%),radial-gradient(circle_at_52%_46%,rgba(135,224,220,0.34),transparent_34%)]" />
        <div className="absolute left-1/2 top-1/2 h-36 w-36 -translate-x-1/2 -translate-y-1/2 rounded-full border border-cyan-100/50" />
        <div className="absolute left-1/2 top-1/2 h-24 w-24 -translate-x-1/2 -translate-y-1/2 rounded-full border border-[#d8b86a]/80" />
        <div className="absolute inset-x-8 top-1/2 h-px bg-cyan-100/55" />
        <div className="absolute inset-y-8 left-1/2 w-px bg-cyan-100/45" />
        <Bolt className="absolute left-[46%] top-[33%] h-16 w-16 text-[#f5cf72]" />
        <Waves className="absolute bottom-8 left-8 h-12 w-12 text-cyan-100/70" />
        <Cloud className="absolute right-8 top-8 h-14 w-14 text-cyan-50/65" />
      </div>
    </div>
  );
}

function WelcomePage({
  learner,
  memories: currentMemories,
}: {
  learner: AuthenticatedStudent;
  memories: StudentMemory[];
}) {
  const interestText =
    learner.interests.length > 0
      ? learner.interests.join(", ")
      : "visual puzzles, ocean analogies, and sketchable explanations";

  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Compass}>Welcome back</Kicker>
      <h2 className="mt-4 text-4xl font-semibold leading-tight text-stone-950">
        Welcome back, {learner.displayName}.
      </h2>
      <p className="mt-4 text-lg leading-8 text-stone-700">
        The Reef remembers what you care about: {interestText}.
      </p>

      <div className="mt-7 space-y-3">
        {currentMemories.slice(0, 4).map((memory) => (
          <div
            key={memory.id}
            className="flex gap-3 border-l-2 border-[#1e6f73] bg-white/45 px-4 py-3"
          >
            <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-[#1e6f73]" />
            <div>
              <p className="text-xs uppercase text-stone-500">{memory.type}</p>
              <p className="text-sm leading-6 text-stone-800">
                {memory.content}
              </p>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-7 rounded-[8px] bg-[#173b3b] p-4 text-stone-50">
        <p className="text-xs uppercase text-cyan-100/75">Today&apos;s quest</p>
        <p className="mt-2 text-xl font-semibold">
          Connect the next lesson to {learner.interests[0] ?? "your interests"}.
        </p>
      </div>
    </div>
  );
}

function StageMapPage({ stages: currentStages }: { stages: Stage[] }) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={MapIcon}>Storm Gate map</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        Three levels guard the chamber.
      </h2>
      <div className="mt-7 space-y-4">
        {currentStages.map((stage, index) => (
          <div key={stage.level} className="grid grid-cols-[42px_1fr] gap-4">
            <div className="flex flex-col items-center">
              <div
                className={`flex h-10 w-10 items-center justify-center rounded-full border text-sm font-semibold ${
                  stage.status === "locked"
                    ? "border-stone-300 bg-stone-100 text-stone-400"
                    : stage.status === "passed"
                      ? "border-[#1e6f73] bg-[#1e6f73] text-white"
                      : "border-[#d8b86a] bg-[#fff8df] text-[#654f12]"
                }`}
              >
                {stage.status === "locked" ? (
                  <Lock className="h-4 w-4" />
                ) : stage.status === "passed" ? (
                  <Check className="h-4 w-4" />
                ) : (
                  index + 1
                )}
              </div>
              {index < currentStages.length - 1 ? (
                <div className="h-14 w-px bg-stone-300" />
              ) : null}
            </div>
            <div>
              <p className="text-lg font-semibold text-stone-950">
                {stage.title}
              </p>
              <p className="mt-1 text-sm leading-6 text-stone-600">
                {stage.description}
              </p>
              <p className="mt-2 text-xs uppercase text-stone-500">
                {stage.status}
              </p>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-auto rounded-[8px] border border-[#d8b86a]/50 bg-[#fff8df] p-4 text-[#514010]">
        Passing Level 1 unlocks the Mechanism Chamber without changing the
        story world.
      </div>
    </div>
  );
}

function AskPage({
  hasAsked,
  lessonStatus,
  suggestedTopics,
  topic,
  onAsk,
  onChooseTopic,
  onTopicChange,
}: {
  hasAsked: boolean;
  lessonStatus: string;
  suggestedTopics: string[];
  topic: string;
  onAsk: () => void;
  onChooseTopic: (topic: string) => void;
  onTopicChange: (topic: string) => void;
}) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Mic}>Ask Primer</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        Choose the next thing to explore.
      </h2>
      <div className="mt-6 rounded-[8px] border border-stone-300 bg-white/62 p-4">
        <label
          htmlFor="topic"
          className="text-xs uppercase text-stone-500"
        >
          Student topic
        </label>
        <input
          id="topic"
          value={topic}
          onChange={(event) => onTopicChange(event.target.value)}
          className="mt-3 h-12 w-full rounded-[8px] border border-stone-300 bg-white px-4 text-base font-semibold text-stone-900 outline-none focus:border-[#1e6f73] focus:ring-2 focus:ring-[#1e6f73]/20"
          placeholder="Try lightning, coral reefs, fractions..."
        />
      </div>
      <div className="mt-5 grid gap-3">
        <button
          type="button"
          onClick={onAsk}
          className="inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#1e6f73] px-5 text-sm font-semibold text-white transition hover:bg-[#195e61]"
        >
          <Volume2 className="h-4 w-4" />
          {hasAsked ? "Guide this topic again" : "Ask Primer to guide me"}
        </button>
      </div>

      <p className="mt-4 text-sm leading-6 text-stone-600">{lessonStatus}</p>

      <div className="mt-6">
        <p className="text-xs uppercase text-stone-500">Platform guidance</p>
        <div className="mt-3 flex flex-wrap gap-2">
          {suggestedTopics.map((suggestedTopic) => (
            <button
              key={suggestedTopic}
              type="button"
              onClick={() => onChooseTopic(suggestedTopic)}
              className="rounded-full border border-stone-300 bg-white/60 px-3 py-2 text-xs font-semibold text-stone-700 transition hover:border-[#1e6f73] hover:text-[#1e6f73]"
            >
              {suggestedTopic}
            </button>
          ))}
        </div>
      </div>

      <p className="mt-auto text-sm leading-6 text-stone-600">
        The backend records progress per student, then asks OpenAI Responses to
        adapt the next explanation to that memory and level.
      </p>
    </div>
  );
}

function StoryPage({ lesson }: { lesson: PrimerLesson }) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Cloud}>Story scene</Kicker>
      <h2 className="mt-4 text-3xl font-semibold leading-tight text-stone-950">
        {lesson.topic}
      </h2>
      <p className="mt-5 text-lg leading-8 text-stone-800">
        {lesson.storyScene}
      </p>
      <div className="mt-6 border-l-2 border-[#1e6f73] bg-white/55 px-4 py-3">
        <p className="text-xs uppercase text-stone-500">Plain explanation</p>
        <p className="mt-2 text-base leading-7 text-stone-800">
          {lesson.plainExplanation}
        </p>
      </div>
      <div className="mt-5 rounded-[8px] bg-[#173b3b] p-4 text-stone-50">
        <p className="text-xs uppercase text-cyan-100/75">Check</p>
        <p className="mt-2 leading-7">{lesson.checkForUnderstanding}</p>
      </div>
    </div>
  );
}

function InfographicPage({
  artifact,
  infographicStatus,
  infographic,
  lesson,
  hasGeneratedInfographic,
  onGenerate,
}: {
  artifact: InfographicArtifact | null;
  infographicStatus: string;
  infographic: InfographicSpec;
  lesson: PrimerLesson;
  hasGeneratedInfographic: boolean;
  onGenerate: () => void;
}) {
  const imageSrc = artifact?.imageDataUrl ?? artifact?.imageUrl ?? null;

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-start justify-between gap-3">
        <Kicker icon={Sparkles}>AI infographic tool</Kicker>
        <button
          type="button"
          onClick={onGenerate}
          className="rounded-full bg-[#d8b86a] px-3 py-1.5 text-xs font-semibold text-[#1c211d]"
        >
          {hasGeneratedInfographic ? "Regenerate" : "Generate"}
        </button>
      </div>
      <h2 className="mt-3 text-2xl font-semibold text-stone-950">
        {lesson.topic} infographic
      </h2>
      <p className="mt-1 text-sm text-stone-600">{infographicStatus}</p>
      {imageSrc ? (
        <div className="mt-4 overflow-hidden rounded-[8px] border border-stone-300 bg-white">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={imageSrc}
            alt={`Generated infographic for ${lesson.topic}`}
            className="aspect-square w-full object-cover"
          />
        </div>
      ) : (
        <>
          <div className="mt-4 rounded-[8px] border border-stone-300 bg-white/70 p-4">
            <p className="text-xs uppercase text-stone-500">
              Prompt sent to gpt-image-2
            </p>
            <p className="mt-2 text-sm leading-6 text-stone-800">
              {lesson.infographicPrompt}
            </p>
          </div>
          <div className="mt-4 grid grid-cols-2 gap-3">
            {infographic.panels.slice(0, 4).map((panel, index) => {
              const Icon = iconMap[panel.icon];

              return (
                <div
                  key={panel.heading}
                  className="min-h-[112px] rounded-[8px] border border-stone-300 bg-white/70 p-3"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex h-8 w-8 items-center justify-center rounded-full bg-[#1e6f73] text-white">
                      <Icon className="h-4 w-4" strokeWidth={1.8} />
                    </div>
                    <span className="text-xs font-semibold text-stone-400">
                      0{index + 1}
                    </span>
                  </div>
                  <p className="mt-2 text-sm font-semibold leading-5 text-stone-950">
                    {panel.heading}
                  </p>
                </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}

function VoiceoverPage({
  lesson,
  isNarrating,
  isNarrationLoading,
  narrationStatus,
  onPlayNarration,
}: {
  lesson: PrimerLesson;
  isNarrating: boolean;
  isNarrationLoading: boolean;
  narrationStatus: string;
  onPlayNarration: () => void;
}) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Volume2}>Voice-over</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        The book reads the diagram aloud.
      </h2>
      <p className="mt-4 text-base leading-7 text-stone-700">
        {lesson.plainExplanation}
      </p>
      <button
        type="button"
        onClick={onPlayNarration}
        disabled={isNarrationLoading}
        className="mt-6 inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#1e6f73] px-5 text-sm font-semibold text-white transition hover:bg-[#195e61] disabled:cursor-wait disabled:opacity-70"
      >
        {isNarrationLoading ? (
          <>
            <Sparkles className="h-4 w-4" />
            Generating narration
          </>
        ) : isNarrating ? (
          <>
            <Square className="h-4 w-4" />
            Stop narration
          </>
        ) : (
          <>
            <Play className="h-4 w-4" />
            Narrate page
          </>
        )}
      </button>
      <p className="mt-3 text-xs leading-5 text-stone-500">
        {narrationStatus} This voice is AI-generated.
      </p>
      <div className="mt-7 grid gap-3">
        {lesson.keyTerms.map((term) => (
          <div
            key={term.term}
            className="rounded-[8px] border border-stone-300 bg-white/60 px-4 py-3"
          >
            <p className="text-sm font-semibold text-stone-950">{term.term}</p>
            <p className="mt-1 text-sm leading-6 text-stone-600">
              {term.definition}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

function FollowUpPage({ lesson }: { lesson: PrimerLesson }) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Waves}>Adaptive follow-up</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        The Primer adjusts its voice.
      </h2>
      <p className="mt-5 text-lg leading-8 text-stone-800">
        {lesson.analogy}
      </p>
      <div className="mt-5 rounded-[8px] border border-stone-300 bg-white/60 p-4">
        <p className="text-xs uppercase text-stone-500">Communication style</p>
        <p className="mt-2 text-sm leading-6 text-stone-700">
          {lesson.communicationStyle}
        </p>
      </div>
      <div className="mt-7 overflow-hidden rounded-[8px] border border-stone-300 bg-white/64">
        <div className="grid grid-cols-3 border-b border-stone-300 bg-stone-100 text-xs font-semibold uppercase text-stone-500">
          <span className="p-3">Term</span>
          <span className="p-3">Memory cue</span>
          <span className="p-3">Physics idea</span>
        </div>
        {lesson.keyTerms.slice(0, 3).map((term) => (
          <div
            key={term.term}
            className="grid grid-cols-3 border-b border-stone-200 text-sm text-stone-700 last:border-b-0"
          >
            <span className="p-3 font-semibold text-stone-900">
              {term.term}
            </span>
            <span className="p-3">Memory-aware</span>
            <span className="p-3">{term.definition}</span>
          </div>
        ))}
      </div>
      <p className="mt-5 text-sm leading-6 text-stone-600">
        This page uses stored learning preferences without inventing new facts
        about the learner.
      </p>
    </div>
  );
}

function StagegatePage({
  answer,
  hasPassed,
  lesson,
  onAnswerChange,
  onSubmit,
}: {
  answer: string;
  hasPassed: boolean;
  lesson: PrimerLesson;
  onAnswerChange: (answer: string) => void;
  onSubmit: () => void;
}) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Unlock}>Stagegate</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        To open the next gate, show your thinking.
      </h2>
      <p className="mt-4 text-sm leading-6 text-stone-600">
        {lesson.stagegatePrompt}
      </p>
      <textarea
        value={answer}
        onChange={(event) => onAnswerChange(event.target.value)}
        className="mt-5 min-h-[150px] resize-none rounded-[8px] border border-stone-300 bg-white/70 p-4 text-sm leading-6 text-stone-800 outline-none focus:border-[#1e6f73] focus:ring-2 focus:ring-[#1e6f73]/20"
      />
      <button
        type="button"
        onClick={onSubmit}
        className="mt-5 inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-5 text-sm font-semibold text-[#1c211d] transition hover:bg-[#e5c879]"
      >
        {hasPassed ? (
          <>
            <Check className="h-4 w-4" />
            Stagegate passed
          </>
        ) : (
          <>
            <Unlock className="h-4 w-4" />
            Submit stagegate
          </>
        )}
      </button>
      <p className="mt-5 text-xs uppercase text-stone-500">
        Passing rule: average rubric score at least 0.75
      </p>
    </div>
  );
}

function UnlockPage({
  result,
  hasPassed,
}: {
  result: StagegateResult;
  hasPassed: boolean;
}) {
  return (
    <div className="flex h-full flex-col text-stone-50">
      <Kicker icon={Check} dark>
        Unlock result
      </Kicker>
      <h2 className="mt-5 text-4xl font-semibold leading-tight">
        {hasPassed ? "The Storm Gate opens." : "The Storm Gate is waiting."}
      </h2>
      <p className="mt-4 text-base leading-7 text-cyan-50/78">
        {hasPassed
          ? result.feedbackToStudent
          : "Submit the stagegate page to unlock the next chamber and add a new visible memory."}
      </p>

      <div className="mt-6 space-y-3">
        {Object.entries(result.rubric).map(([label, value]) => (
          <RubricBar key={label} label={label} value={hasPassed ? value : 0} />
        ))}
      </div>

      <div className="mt-6 rounded-[8px] border border-cyan-100/20 bg-cyan-50/10 p-4">
        <p className="text-xs uppercase text-cyan-100/75">New memory</p>
        <p className="mt-2 text-sm leading-6">
          {hasPassed
            ? unlockedMemory.content
            : "No new mastery memory has been added yet."}
        </p>
      </div>

      <div className="mt-auto rounded-[8px] bg-[#d8b86a] p-4 text-[#1c211d]">
        <p className="text-xs uppercase">Unlocked</p>
        <p className="mt-1 text-lg font-semibold">
          {hasPassed ? "Level 2: The Mechanism Chamber" : "Pass Level 1 first"}
        </p>
      </div>
    </div>
  );
}

function MemoryGraphDialog({
  graph,
  status,
  selectedNodeId,
  onClose,
  onRefresh,
  onWalkNode,
}: {
  graph: StudentMemoryGraph | null;
  status: string;
  selectedNodeId: string | null;
  onClose: () => void;
  onRefresh: () => void;
  onWalkNode: (nodeId: string) => void;
}) {
  const flowNodes = useMemo(
    () => (graph ? toMemoryFlowNodes(graph, selectedNodeId) : []),
    [graph, selectedNodeId],
  );
  const flowEdges = useMemo(
    () => (graph ? toMemoryFlowEdges(graph, selectedNodeId) : []),
    [graph, selectedNodeId],
  );
  const [nodes, setNodes, onNodesChange] =
    useNodesState<MemoryFlowNode>(flowNodes);
  const [edges, setEdges, onEdgesChange] =
    useEdgesState<MemoryFlowEdge>(flowEdges);

  useEffect(() => {
    setNodes((currentNodes) =>
      preserveDraggedPositions(flowNodes, currentNodes),
    );
  }, [flowNodes, setNodes]);

  useEffect(() => {
    setEdges(flowEdges);
  }, [flowEdges, setEdges]);

  const handleNodeClick: NodeMouseHandler<MemoryFlowNode> = (_, node) => {
    onWalkNode(node.id);
  };

  return (
    <div className="fixed inset-0 z-50 bg-[#071111]/94 text-stone-100 backdrop-blur-md">
      <div className="flex h-full min-h-0 flex-col px-3 py-3 sm:px-5 sm:py-5">
        <header className="flex flex-wrap items-center justify-between gap-3 border-b border-cyan-100/12 pb-3">
          <div>
            <p className="inline-flex items-center gap-2 text-xs uppercase text-[#d8b86a]">
              <Brain className="h-3.5 w-3.5" />
              Memory
            </p>
            <h2 className="mt-1 text-2xl font-semibold text-cyan-50">
              Student memory graph
            </h2>
            <p className="mt-1 text-sm text-stone-400">{status}</p>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onRefresh}
              className="inline-flex h-10 items-center rounded-full border border-cyan-100/15 px-4 text-sm text-cyan-50/80 transition hover:text-cyan-50"
            >
              Refresh
            </button>
            <button
              type="button"
              onClick={onClose}
              aria-label="Close memory graph"
              className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-cyan-100/15 text-cyan-50/80 transition hover:text-cyan-50"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </header>

        <div className="min-h-0 flex-1 pt-4">
          <div className="memory-flow h-full min-h-[520px] overflow-hidden rounded-[8px] border border-cyan-100/12 bg-[#0c1717]">
            {graph ? (
              <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onNodeClick={handleNodeClick}
                nodesDraggable
                nodesConnectable={false}
                edgesFocusable
                fitView
                fitViewOptions={{ padding: 0.22 }}
                proOptions={{ hideAttribution: true }}
              >
                <Background color="#244848" gap={26} />
                <MiniMap
                  pannable
                  zoomable
                  nodeColor={(node) =>
                    node.data.nodeType === "value" ? "#d8b86a" : "#46a7aa"
                  }
                  maskColor="rgb(7 17 17 / 0.72)"
                />
                <Controls showInteractive={false} />
              </ReactFlow>
            ) : (
              <div className="flex h-full min-h-[520px] items-center justify-center px-6 text-center text-sm text-stone-400">
                Loading relational bitemporal memory for this student.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function Kicker({
  icon: Icon,
  children,
  dark = false,
}: {
  icon: ComponentType<{ className?: string }>;
  children: ReactNode;
  dark?: boolean;
}) {
  return (
    <p
      className={`inline-flex items-center gap-2 text-xs uppercase ${
        dark ? "text-cyan-50/75" : "text-[#1e6f73]"
      }`}
    >
      <Icon className="h-3.5 w-3.5" />
      {children}
    </p>
  );
}

function RubricBar({ label, value }: { label: string; value: number }) {
  const percent = Math.round(value * 100);
  const readableLabel = label.replace(/([A-Z])/g, " $1").toLowerCase();

  return (
    <div>
      <div className="flex items-center justify-between text-xs uppercase text-cyan-50/72">
        <span>{readableLabel}</span>
        <span>{percent}%</span>
      </div>
      <div className="mt-1 h-2 overflow-hidden rounded-full bg-cyan-50/12">
        <div
          className="h-full rounded-full bg-[#d8b86a] transition-all duration-700"
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
}

function mergeMemoryGraph(
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

function toMemoryFlowNodes(
  graph: StudentMemoryGraph,
  selectedNodeId: string | null,
): MemoryFlowNode[] {
  return graph.nodes.map((node, index) => {
    const isRoot = node.id === graph.rootNodeId;
    const isValue = node.nodeType === "value";
    const isSelected = node.id === selectedNodeId;

    return {
      id: node.id,
      type: "default",
      position: memoryNodePosition(node, index, graph.nodes.length, isRoot),
      selected: isSelected,
      draggable: true,
      data: {
        label: node.label,
        kind: node.kind,
        nodeType: node.nodeType,
        factCount: node.factCount,
        expanded: node.expanded,
        summary: node.summary,
      },
      className: `memory-flow-node ${
        isSelected ? "memory-flow-node-selected" : ""
      } ${isRoot ? "memory-flow-node-root" : ""} ${
        isValue ? "memory-flow-node-value" : ""
      }`,
    };
  });
}

function memoryNodePosition(
  node: MemoryGraphNodeRecord,
  index: number,
  total: number,
  isRoot: boolean,
) {
  if (isRoot) {
    return { x: 0, y: 0 };
  }

  const radius = node.nodeType === "value" ? 420 : 290;
  const angle =
    ((Math.max(index, 1) - 1) / Math.max(total - 1, 1)) * Math.PI * 2 -
    Math.PI / 2;

  return {
    x: Math.round(Math.cos(angle) * radius),
    y: Math.round(Math.sin(angle) * radius),
  };
}

function toMemoryFlowEdges(
  graph: StudentMemoryGraph,
  selectedNodeId: string | null,
): MemoryFlowEdge[] {
  return graph.edges.map((edge) => {
    const isSelected =
      edge.source === selectedNodeId || edge.target === selectedNodeId;

    return {
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: "smoothstep",
      label: edge.label,
      animated: isSelected,
      data: {
        label: edge.label,
        content: edge.content,
        memoryType: edge.memoryType,
        confidence: edge.confidence,
      },
      style: {
        stroke: isSelected ? "#d8b86a" : "#4f8c8f",
        strokeWidth: isSelected ? 2.4 : 1.5,
      },
      labelStyle: {
        fill: "#d8b86a",
        fontSize: 11,
        fontWeight: 600,
      },
      labelBgStyle: {
        fill: "#071111",
        fillOpacity: 0.86,
      },
      labelBgPadding: [6, 3],
      labelBgBorderRadius: 4,
    };
  });
}

function preserveDraggedPositions(
  nextNodes: MemoryFlowNode[],
  currentNodes: MemoryFlowNode[],
): MemoryFlowNode[] {
  const positions = new Map(
    currentNodes.map((node) => [node.id, node.position] as const),
  );
  return nextNodes.map((node) => ({
    ...node,
    position: positions.get(node.id) ?? node.position,
  }));
}

function normalizeMemoryGraph(value: unknown): StudentMemoryGraph | null {
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

function normalizeLesson(value: unknown, fallbackTopic: string): PrimerLesson {
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

function normalizeStagegateResult(value: unknown): StagegateResult {
  const record = asRecord(value);
  if (!record) {
    return seededStagegateResult;
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
      "The Primer could not grade this answer yet.",
    nextLevelUnlocked: nextLevelField(record, "nextLevelUnlocked"),
  };
}

function normalizeMemories(value: unknown): StudentMemory[] | null {
  if (!Array.isArray(value)) {
    return null;
  }

  const normalized = value
    .map((item, index): StudentMemory | null => {
      const memory = asRecord(item) as BackendMemory | null;
      const content = memory?.content;
      if (!content) {
        return null;
      }

      const normalizedMemory: StudentMemory = {
        id: memory.assertionId ?? `${content}-${index}`,
        type: memory.memory_type ?? memory.memoryType ?? "knowledge",
        content,
        tags: Array.isArray(memory.tags) ? memory.tags : [],
        assertionId: memory.assertionId,
        subject: memory.subject,
        predicate: memory.predicate,
        validFrom: memory.validFrom,
        validTo: memory.validTo,
        knownFrom: memory.knownFrom,
        knownTo: memory.knownTo,
        source: memory.source,
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

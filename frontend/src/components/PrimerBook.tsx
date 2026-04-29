"use client";

import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node as ReactFlowNode,
  type NodeMouseHandler,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import {
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
  Lock,
  LogOut,
  Map as MapIcon,
  Maximize2,
  Mic,
  Play,
  RotateCcw,
  Sparkles,
  Square,
  TriangleAlert,
  Unlock,
  Volume2,
  Waves,
  X,
} from "lucide-react";
import { AuthGate } from "@/components/AuthGate";
import { LessonGenerationOverlay } from "@/components/LessonGenerationOverlay";
import { ReportCardDialog } from "@/components/ReportCardPages";
import HTMLFlipBook from "react-pageflip";
import {
  type AuthPayload,
  type AuthSession,
  type AuthenticatedStudent,
  authHeaders,
  apiBaseUrl,
  clearStoredAuth,
  normalizeAuthenticatedStudent,
  readStoredAuth,
  storeAuth,
} from "@/lib/auth";
import {
  isPointerNearPageTurnEdge,
  normalizeSelectedBookText,
  selectedTextInfographicPrompt,
} from "@/lib/book-selection";
import {
  type LessonStartPayload,
  type MemoryGraphNodeRecord,
  type PrimerLesson,
  type Stage,
  type StagegateResult,
  type StudentBookLesson,
  type StudentBookState,
  type StudentMemory,
  type StudentMemoryGraph,
  buildLessonStartBody,
  currentBookLessonIndex,
  emptyStagegateResult,
  firstTopicHint,
  initialLesson,
  lessonStagegatePageIndex,
  lessonStoryPageIndex,
  nextTopicPageIndexForLessonCount,
  mergeMemoryGraph,
  normalizeBookState,
  normalizeLesson,
  normalizeMemoryGraph,
  normalizeMemories,
  normalizeStagegateResult,
  orderedBookLessons,
  openingProfileHint,
  pageCountForLessonCount,
  restoredBookTargetPage,
  stagesForStagegate,
  unlockPageIndexForLessonCount,
} from "@/lib/primer-flow";
import {
  type StudentReportCard,
  fetchStudentReportCard,
} from "@/lib/report-card";
import {
  type ComponentType,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
  type TouchEvent as ReactTouchEvent,
  forwardRef,
  useCallback,
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
  turnToPage?: (page: number) => void;
  update?: () => void;
  getPageCount: () => number;
  getCurrentPageIndex: () => number;
};

type FlipBookRef = {
  pageFlip: () => PageFlipApi | undefined;
};

type PageFlipEvent<T> = {
  data: T;
};

type InfographicArtifact = {
  aiMode?: string;
  error?: string;
  generated?: boolean;
  imageDataUrl?: string | null;
  imageUrl?: string | null;
  message?: string;
  prompt?: string;
  model?: string;
};

type SelectionInfographic = {
  artifact: InfographicArtifact;
  id: string;
  pageIndex: number;
  sourceText: string;
  topic: string;
};

type SelectedTextAction = {
  pageIndex: number;
  text: string;
  x: number;
  y: number;
};

type EnlargedInfographic = {
  alt: string;
  src: string;
  title?: string;
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

type InfographicExplanationPayload = {
  aiMode?: string;
  cached?: boolean;
  error?: string;
  explanation?: string;
  generated?: boolean;
  keyObservations?: string[];
  message?: string;
  model?: string;
  persistedVoiceover?: {
    cacheKey?: string;
    contentType?: string;
    error?: string;
    filePath?: string;
    reused?: boolean;
    saved?: boolean;
  };
  speech?: NarrationPayload | null;
  speechGenerated?: boolean;
  speechModel?: string;
  spokenExplanation?: string;
  voice?: string;
};

type ResetStudentPayload = {
  error?: string;
  student?: AuthenticatedStudent | null;
  studentId?: string;
};

type StudentBookPayload = {
  book: StudentBookState | null;
  student: AuthenticatedStudent | null;
};

type DisplayedBookLesson = {
  bookLesson: StudentBookLesson | null;
  isCurrent: boolean;
  lesson: PrimerLesson;
  lessonId: string;
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

type MemoryFlowNode = ReactFlowNode<MemoryNodeData>;
type MemoryFlowEdge = Edge<MemoryEdgeData>;

type BookPageProps = {
  children: ReactNode;
  density?: "hard" | "soft";
  inlineInfographics?: SelectionInfographic[];
  onLayoutChange?: () => void;
  onOpenInfographic?: (image: EnlargedInfographic) => void;
  pageIndex: number;
  pageNumber?: number;
  tone?: "cover" | "paper" | "deep";
};

const BookPage = forwardRef<HTMLDivElement, BookPageProps>(
  (
    {
      children,
      density = "soft",
      inlineInfographics = [],
      onLayoutChange,
      onOpenInfographic,
      pageIndex,
      pageNumber,
      tone = "paper",
    },
    ref,
  ) => {
    return (
      <section
        ref={ref}
        data-density={density}
        data-primer-page-index={pageIndex}
        onMouseDownCapture={guardPageFlipGesture}
        onTouchStartCapture={guardPageFlipGesture}
        className={`primer-page primer-page-${tone}`}
      >
        <div className="primer-page-grain" />
        <div className="relative z-10 flex h-full flex-col overflow-hidden px-7 py-7 sm:px-8 sm:py-8">
          <div className="min-h-0 flex-1 overflow-hidden">{children}</div>
          {inlineInfographics.length > 0 ? (
            <InlineSelectionInfographics
              infographics={inlineInfographics}
              onLayoutChange={onLayoutChange}
              onOpenInfographic={onOpenInfographic}
            />
          ) : null}
          {typeof pageNumber === "number" ? (
            <div className="mt-auto flex items-center justify-between border-t border-stone-300/70 pt-3 text-[11px] uppercase text-stone-500">
              <span>Primer</span>
              <span>{pageNumber}</span>
            </div>
          ) : null}
        </div>
      </section>
    );
  },
);

BookPage.displayName = "BookPage";

type PageFlipGestureEvent =
  | ReactMouseEvent<HTMLElement>
  | ReactTouchEvent<HTMLElement>;

function gestureClientX(event: PageFlipGestureEvent): number | null {
  if ("touches" in event) {
    return event.touches[0]?.clientX ?? event.changedTouches[0]?.clientX ?? null;
  }

  return event.clientX;
}

function guardPageFlipGesture(event: PageFlipGestureEvent) {
  const clientX = gestureClientX(event);
  const bookBlock = event.currentTarget
    .closest(".primer-book-shell")
    ?.querySelector(".stf__block");

  if (!bookBlock || clientX === null) {
    event.stopPropagation();
    return;
  }

  if (!isPointerNearPageTurnEdge(clientX, bookBlock.getBoundingClientRect())) {
    event.stopPropagation();
  }
}

function stopPageFlipGesture(event: PageFlipGestureEvent) {
  event.stopPropagation();
}

const pageFlipInteractiveHandlers = {
  onMouseDownCapture: stopPageFlipGesture,
  onTouchStartCapture: stopPageFlipGesture,
};

const pageTurnStyle: CSSProperties = {};

const infographicSparkles = [
  { x: "16%", y: "24%", delay: "0s", size: "0.42rem" },
  { x: "32%", y: "16%", delay: "0.16s", size: "0.56rem" },
  { x: "52%", y: "20%", delay: "0.34s", size: "0.36rem" },
  { x: "76%", y: "26%", delay: "0.5s", size: "0.5rem" },
  { x: "22%", y: "48%", delay: "0.68s", size: "0.34rem" },
  { x: "43%", y: "42%", delay: "0.86s", size: "0.66rem" },
  { x: "65%", y: "48%", delay: "1.04s", size: "0.4rem" },
  { x: "84%", y: "58%", delay: "1.2s", size: "0.52rem" },
  { x: "18%", y: "76%", delay: "1.38s", size: "0.48rem" },
  { x: "39%", y: "82%", delay: "1.54s", size: "0.32rem" },
  { x: "60%", y: "74%", delay: "1.72s", size: "0.58rem" },
  { x: "78%", y: "82%", delay: "1.9s", size: "0.38rem" },
];

async function requestLessonStart(
  learner: AuthenticatedStudent,
  session: AuthSession | null,
  nextTopic?: string,
  signal?: AbortSignal,
): Promise<LessonStartPayload> {
  const body = buildLessonStartBody(learner, nextTopic);

  const response = await fetch(`${apiBaseUrl}/api/lesson/start`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeaders(session) },
    body: JSON.stringify(body),
    signal,
  });

  return (await response.json()) as LessonStartPayload;
}

async function fetchStudentBookState(
  learner: AuthenticatedStudent,
  session: AuthSession | null,
  signal?: AbortSignal,
): Promise<StudentBookPayload> {
  const response = await fetch(`${apiBaseUrl}/api/book/${learner.studentId}`, {
    method: "GET",
    headers: authHeaders(session),
    signal,
  });
  const payload = (await response.json()) as {
    book?: unknown;
    student?: unknown;
  };
  return {
    book: normalizeBookState(payload.book),
    student: normalizeAuthenticatedStudent(payload.student),
  };
}

async function resetStudentRecord(
  learner: AuthenticatedStudent,
  session: AuthSession | null,
): Promise<ResetStudentPayload> {
  const response = await fetch(
    `${apiBaseUrl}/api/students/${encodeURIComponent(learner.studentId)}/reset`,
    {
      method: "POST",
      headers: authHeaders(session),
    },
  );

  return (await response.json()) as ResetStudentPayload;
}

function displayedLessonsForBook(
  book: StudentBookState | null,
  currentLesson: PrimerLesson,
  learner: AuthenticatedStudent | null,
): DisplayedBookLesson[] {
  const fallbackTopic = learner ? firstTopicHint(learner) : currentLesson.topic;
  const bookLessons = orderedBookLessons(book?.lessons ?? []);
  if (bookLessons.length === 0) {
    return [
      {
        bookLesson: null,
        isCurrent: true,
        lesson: currentLesson,
        lessonId: "current-draft-lesson",
      },
    ];
  }

  const currentLessonIndex = currentBookLessonIndex(book);
  return bookLessons.map((bookLesson, index) => ({
    bookLesson,
    isCurrent: index === currentLessonIndex,
    lesson: normalizeLesson(bookLesson.lesson, bookLesson.topic || fallbackTopic),
    lessonId: bookLesson.lessonId,
  }));
}

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
  const bookShellRef = useRef<HTMLDivElement | null>(null);
  const narrationAudioRef = useRef<HTMLAudioElement | null>(null);
  const bootstrappedStudentIdRef = useRef<string | null>(null);
  const infographicExplanationRequestIdRef = useRef(0);
  const layoutRefreshTimerRef = useRef<number[]>([]);
  const pendingBookEndPageRef = useRef<number | null>(null);
  const selectionCaptureTimerRef = useRef<number | null>(null);
  const [authChecked, setAuthChecked] = useState(false);
  const [authenticatedStudent, setAuthenticatedStudent] =
    useState<AuthenticatedStudent | null>(null);
  const [session, setSession] = useState<AuthSession | null>(null);
  const [currentPage, setCurrentPage] = useState(0);
  const [topic, setTopic] = useState("");
  const [lesson, setLesson] = useState<PrimerLesson>(initialLesson);
  const [persistedBook, setPersistedBook] = useState<StudentBookState | null>(
    null,
  );
  const [remoteMemories, setRemoteMemories] = useState<StudentMemory[] | null>(
    null,
  );
  const [reportCard, setReportCard] = useState<StudentReportCard | null>(null);
  const [reportCardStatus, setReportCardStatus] = useState(
    "Report card not loaded.",
  );
  const [isReportCardRefreshing, setIsReportCardRefreshing] = useState(false);
  const [isReportOpen, setIsReportOpen] = useState(false);
  const [memoryGraph, setMemoryGraph] = useState<StudentMemoryGraph | null>(
    null,
  );
  const [isMemoryOpen, setIsMemoryOpen] = useState(false);
  const [isResetDialogOpen, setIsResetDialogOpen] = useState(false);
  const [isResettingStudent, setIsResettingStudent] = useState(false);
  const [resetError, setResetError] = useState<string | null>(null);
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
    "A starting point will be chosen from the student profile.",
  );
  const [infographicStatus, setInfographicStatus] = useState(
    "No generated infographic yet.",
  );
  const [isInfographicGenerating, setIsInfographicGenerating] = useState(false);
  const [infographicArtifact, setInfographicArtifact] =
    useState<InfographicArtifact | null>(null);
  const [selectionInfographics, setSelectionInfographics] = useState<
    SelectionInfographic[]
  >([]);
  const [selectedTextAction, setSelectedTextAction] =
    useState<SelectedTextAction | null>(null);
  const [isSelectionInfographicLoading, setIsSelectionInfographicLoading] =
    useState(false);
  const [enlargedInfographic, setEnlargedInfographic] =
    useState<EnlargedInfographic | null>(null);
  const [infographicExplanation, setInfographicExplanation] =
    useState<InfographicExplanationPayload | null>(null);
  const [isInfographicExplanationLoading, setIsInfographicExplanationLoading] =
    useState(false);
  const [isInfographicExplanationPlaying, setIsInfographicExplanationPlaying] =
    useState(false);
  const [infographicExplanationStatus, setInfographicExplanationStatus] =
    useState("Ready to explain the enlarged diagram.");
  const [stagegateResult, setStagegateResult] =
    useState<StagegateResult>(emptyStagegateResult);
  const [isStagegateSubmitting, setIsStagegateSubmitting] = useState(false);
  const [isLessonGenerating, setIsLessonGenerating] = useState(false);
  const [answer, setAnswer] = useState("");
  const [isNarrating, setIsNarrating] = useState(false);
  const [isNarrationLoading, setIsNarrationLoading] = useState(false);
  const [narrationStatus, setNarrationStatus] = useState(
    "OpenAI narration is ready.",
  );

  const syncAuthenticatedStudentSnapshot = useCallback((
    student: unknown,
    nextSession: AuthSession | null = session,
  ): AuthenticatedStudent | null => {
    const normalizedStudent = normalizeAuthenticatedStudent(student);
    if (!normalizedStudent) {
      return null;
    }

    setAuthenticatedStudent(normalizedStudent);
    storeAuth({ student: normalizedStudent, session: nextSession });
    return normalizedStudent;
  }, [session]);

  const hydratePersistedBook = useCallback(
    (
      book: StudentBookState | null,
      learner: AuthenticatedStudent,
      options?: { turnToEnd?: boolean },
    ) => {
      if (!book) {
        return false;
      }

      setPersistedBook(book);
      if (options?.turnToEnd) {
        pendingBookEndPageRef.current = restoredBookTargetPage(book);
      }
      if (book.currentLesson) {
        const savedLesson = normalizeLesson(
          book.currentLesson,
          firstTopicHint(learner),
        );
        setLesson(savedLesson);
        setTopic(savedLesson.topic);
        setHasAsked(true);
        setIsLessonGenerating(false);
      }

      const infographic = book.latestInfographic as
        | InfographicArtifact
        | undefined;
      setInfographicArtifact(infographic ?? null);
      setHasGeneratedInfographic(Boolean(infographic?.generated));
      setInfographicStatus(
        infographic
          ? infographic.generated
            ? `Generated with ${infographic.model ?? "gpt-image-2"}.`
            : (infographic.message ?? "Infographic fallback was saved.")
          : "No generated infographic yet.",
      );

      if (book.latestStagegate) {
        const savedStagegate = normalizeStagegateResult(book.latestStagegate);
        setStagegateResult(savedStagegate);
        setHasPassedStagegate(book.hasPassedStagegate || savedStagegate.passed);
      } else {
        setStagegateResult(emptyStagegateResult);
        setHasPassedStagegate(false);
      }
      setAnswer(book.latestAnswer ?? "");

      return Boolean(book.currentLesson);
    },
    [],
  );

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
        setTopic("");
      }
      setAuthChecked(true);
    });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const learner = authenticatedStudent;
    if (!learner) {
      return;
    }

    if (bootstrappedStudentIdRef.current === learner.studentId) {
      return;
    }

    bootstrappedStudentIdRef.current = learner.studentId;
    const controller = new AbortController();
    let cancelled = false;

    setHasAsked(true);
    setHasGeneratedInfographic(false);
    setHasPassedStagegate(false);
    setIsStagegateSubmitting(false);
    setIsLessonGenerating(false);
    setIsInfographicGenerating(false);
    setInfographicArtifact(null);
    setSelectionInfographics([]);
    setSelectedTextAction(null);
    setStagegateResult(emptyStagegateResult);
    setAnswer("");
    setReportCard(null);
    setReportCardStatus("Report card not loaded.");
    setIsReportOpen(false);
    setLessonStatus("Loading the saved Primer book...");

    void fetchStudentBookState(learner, session, controller.signal)
      .then((savedPayload) => {
        if (cancelled) {
          return null;
        }

        const savedStudent = normalizeAuthenticatedStudent(savedPayload.student);
        if (savedStudent) {
          // Keep the refreshed DB profile without aborting the bootstrap lesson request.
          storeAuth({ student: savedStudent, session });
          setRemoteMemories(normalizeMemories(savedStudent.memories));
        }
        const restoredStudent = savedStudent ?? learner;

        if (
          hydratePersistedBook(savedPayload.book, restoredStudent, {
            turnToEnd: true,
          })
        ) {
          syncAuthenticatedStudentSnapshot(savedPayload.student);
          setLessonStatus("Restored the saved Primer book from the database.");
          return null;
        }

        setLessonStatus("Asking OpenAI Responses to choose the opening path...");
        setIsLessonGenerating(true);
        return requestLessonStart(
          restoredStudent,
          session,
          undefined,
          controller.signal,
        );
      })
      .then((payload) => {
        if (cancelled || !payload) {
          return;
        }

        const responseStudent =
          syncAuthenticatedStudentSnapshot(payload.student) ?? learner;
        setRemoteMemories(normalizeMemories(responseStudent.memories));
        if (payload.error || !payload.lesson) {
          setLessonStatus(
            payload.error ?? "The opening lesson could not be generated yet.",
          );
          return;
        }

        const normalizedLesson = normalizeLesson(
          payload.lesson,
          firstTopicHint(responseStudent),
        );
        setLesson(normalizedLesson);
        setTopic(normalizedLesson.topic);
        hydratePersistedBook(normalizeBookState(payload.book), responseStudent, {
          turnToEnd: true,
        });
        setLessonStatus(
          normalizedLesson.aiMode === "openai_responses"
            ? "Opening path generated by OpenAI Responses."
            : "Opening path generated from the student profile.",
        );
      })
      .catch((error) => {
        if (!cancelled && error instanceof DOMException && error.name === "AbortError") {
          return;
        }

        if (!cancelled) {
          setLessonStatus(`Could not reach backend: ${String(error)}`);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLessonGenerating(false);
        }
      });

    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [
    authenticatedStudent,
    hydratePersistedBook,
    session,
    syncAuthenticatedStudentSnapshot,
  ]);

  useEffect(() => {
    return () => {
      for (const timer of layoutRefreshTimerRef.current) {
        window.clearTimeout(timer);
      }
      layoutRefreshTimerRef.current = [];
      if (selectionCaptureTimerRef.current !== null) {
        window.clearTimeout(selectionCaptureTimerRef.current);
      }
      const audio = narrationAudioRef.current;
      if (audio) {
        audio.pause();
        audio.src = "";
      }
      narrationAudioRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!isInfographicExplanationLoading) {
      return;
    }

    const messages = [
      "Reading the diagram with GPT-5.5 vision...",
      "Writing a learner-friendly explanation...",
      "Generating fable voice narration...",
    ];
    let index = 0;
    const timer = window.setInterval(() => {
      index = (index + 1) % messages.length;
      setInfographicExplanationStatus(messages[index]);
    }, 1800);

    return () => window.clearInterval(timer);
  }, [isInfographicExplanationLoading]);

  async function loadReportCard(
    options: {
      learner?: AuthenticatedStudent;
      session?: AuthSession | null;
    } = {},
  ) {
    const learner = options.learner ?? authenticatedStudent;
    if (!learner) {
      return;
    }

    const reportSession = "session" in options ? options.session ?? null : session;

    setIsReportCardRefreshing(true);
    setReportCardStatus("Refreshing report card...");

    try {
      const nextReport = await fetchStudentReportCard(learner, reportSession);
      if (!nextReport) {
        setReportCard(null);
        setReportCardStatus("No report-card evidence is available yet.");
        return;
      }

      setReportCard(nextReport);
      setReportCardStatus(
        nextReport.aiMode === "openai_responses"
          ? "Report card refreshed with OpenAI narrative."
          : "Report card refreshed from saved learning evidence.",
      );
    } catch (error) {
      setReportCardStatus(`Could not refresh report card: ${String(error)}`);
    } finally {
      setIsReportCardRefreshing(false);
    }
  }

  function openReportCard() {
    setIsReportOpen(true);
    void loadReportCard();
  }

  useEffect(() => {
    const learner = authenticatedStudent;
    if (!learner) {
      return;
    }

    const controller = new AbortController();
    let cancelled = false;
    const loadingTimer = window.setTimeout(() => {
      if (!cancelled) {
        setReportCardStatus("Loading report card...");
      }
    }, 0);

    void fetchStudentReportCard(learner, session, controller.signal)
      .then((nextReport) => {
        if (cancelled) {
          return;
        }
        setReportCard(nextReport);
        setReportCardStatus(
          nextReport
            ? "Report card loaded from saved learning evidence."
            : "No report-card evidence is available yet.",
        );
      })
      .catch((error) => {
        if (cancelled || (error instanceof DOMException && error.name === "AbortError")) {
          return;
        }
        setReportCardStatus(`Could not load report card: ${String(error)}`);
      });

    return () => {
      cancelled = true;
      window.clearTimeout(loadingTimer);
      controller.abort();
    };
  }, [authenticatedStudent, session]);

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
    return remoteMemories ?? [];
  }, [remoteMemories]);

  const displayedBookLessons = useMemo(
    () => displayedLessonsForBook(persistedBook, lesson, authenticatedStudent),
    [authenticatedStudent, lesson, persistedBook],
  );

  const lessonEntryCount = displayedBookLessons.length;
  const currentDisplayedLessonIndex = Math.max(
    0,
    displayedBookLessons.findIndex((entry) => entry.isCurrent),
  );
  const currentStoryPageIndex = lessonStoryPageIndex(
    currentDisplayedLessonIndex,
  );
  const currentStagegatePageIndex = lessonStagegatePageIndex(
    currentDisplayedLessonIndex,
  );
  const unlockPageIndex = unlockPageIndexForLessonCount(lessonEntryCount);
  const nextTopicPageIndex =
    nextTopicPageIndexForLessonCount(lessonEntryCount);
  const pageCount = pageCountForLessonCount(lessonEntryCount);
  const flipBookStructureKey = displayedBookLessons
    .map((entry) => entry.lessonId)
    .join("|");

  const visibleStages = useMemo<Stage[]>(() => {
    return stagesForStagegate(hasPassedStagegate);
  }, [hasPassedStagegate]);

  const isNextTopicUnlocked = hasPassedStagegate;
  const selectionInfographicsByPage = useMemo(() => {
    const byPage = new Map<number, SelectionInfographic[]>();
    for (const infographic of selectionInfographics) {
      const pageInfographics = byPage.get(infographic.pageIndex) ?? [];
      pageInfographics.push(infographic);
      byPage.set(infographic.pageIndex, pageInfographics);
    }
    return byPage;
  }, [selectionInfographics]);

  useEffect(() => {
    const targetPage = pendingBookEndPageRef.current;
    if (targetPage === null || !authenticatedStudent) {
      return;
    }

    const clampedPage = Math.max(0, Math.min(targetPage, pageCount - 1));
    const timers: number[] = [];

    function turnWhenReady(attemptsLeft: number) {
      const pageFlip = bookRef.current?.pageFlip();
      const availablePageCount = pageFlip?.getPageCount() ?? pageCount;
      if (pageFlip && availablePageCount > clampedPage) {
        pendingBookEndPageRef.current = null;
        pageFlip.flip(clampedPage, "bottom");
        setCurrentPage(clampedPage);
        return;
      }

      if (attemptsLeft > 0) {
        timers.push(
          window.setTimeout(() => turnWhenReady(attemptsLeft - 1), 80),
        );
      }
    }

    timers.push(window.setTimeout(() => turnWhenReady(6), 0));

    return () => {
      for (const timer of timers) {
        window.clearTimeout(timer);
      }
    };
  }, [authenticatedStudent, pageCount]);

  function flipNext() {
    bookRef.current?.pageFlip()?.flipNext("bottom");
  }

  function flipPrev() {
    bookRef.current?.pageFlip()?.flipPrev("bottom");
  }

  function goToPage(page: number) {
    bookRef.current?.pageFlip()?.flip(page, "bottom");
  }

  function clearBookLayoutRefreshTimers() {
    for (const timer of layoutRefreshTimerRef.current) {
      window.clearTimeout(timer);
    }
    layoutRefreshTimerRef.current = [];
  }

  function refreshBookLayout(targetPage?: number) {
    const pageFlip = bookRef.current?.pageFlip();
    if (!pageFlip) {
      return;
    }

    pageFlip.update?.();

    if (typeof targetPage !== "number") {
      return;
    }

    const pageCount = pageFlip.getPageCount();
    if (pageCount <= 0) {
      return;
    }

    const clampedPage = Math.max(0, Math.min(targetPage, pageCount - 1));
    pageFlip.turnToPage?.(clampedPage);
    setCurrentPage(clampedPage);
  }

  function scheduleBookLayoutRefresh(targetPage?: number) {
    clearBookLayoutRefreshTimers();
    layoutRefreshTimerRef.current = [0, 80, 240].map((delay) =>
      window.setTimeout(() => refreshBookLayout(targetPage), delay),
    );
  }

  function inlineInfographicsForPage(pageIndex: number) {
    return selectionInfographicsByPage.get(pageIndex) ?? [];
  }

  function openInfographicImage(image: EnlargedInfographic) {
    infographicExplanationRequestIdRef.current += 1;
    setEnlargedInfographic(image);
    setInfographicExplanation(null);
    setIsInfographicExplanationLoading(false);
    setIsInfographicExplanationPlaying(false);
    setInfographicExplanationStatus("Ready to explain the enlarged diagram.");
  }

  function scheduleSelectionCapture() {
    if (selectionCaptureTimerRef.current !== null) {
      window.clearTimeout(selectionCaptureTimerRef.current);
    }

    selectionCaptureTimerRef.current = window.setTimeout(() => {
      selectionCaptureTimerRef.current = null;
      captureSelectedBookText();
    }, 0);
  }

  function captureSelectedBookText() {
    const shell = bookShellRef.current;
    const selection = window.getSelection();
    if (!shell || !selection || selection.rangeCount === 0 || selection.isCollapsed) {
      setSelectedTextAction(null);
      return;
    }

    const range = selection.getRangeAt(0);
    const selectionRoot = elementFromNode(range.commonAncestorContainer);
    if (!selectionRoot || !shell.contains(selectionRoot)) {
      setSelectedTextAction(null);
      return;
    }

    const text = normalizeSelectedBookText(selection.toString());
    const bounds = firstRangeBounds(range);
    if (!text || !bounds) {
      setSelectedTextAction(null);
      return;
    }

    const position = popupPositionForBounds(bounds);
    setSelectedTextAction({
      pageIndex: pageIndexFromRange(range) ?? currentPage,
      text,
      x: position.x,
      y: position.y,
    });
  }

  async function generateInfographicForSelection() {
    if (
      !authenticatedStudent ||
      !selectedTextAction ||
      isSelectionInfographicLoading ||
      isInfographicGenerating
    ) {
      return;
    }

    const action = selectedTextAction;
    setIsSelectionInfographicLoading(true);
    setInfographicStatus("Calling gpt-image-2 for the selected passage...");

    try {
      const response = await fetch(`${apiBaseUrl}/api/artifact/infographic`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders(session) },
        body: JSON.stringify({
          studentId: authenticatedStudent.studentId,
          topic: lesson.topic,
          lessonSummary: action.text,
          infographicPrompt: selectedTextInfographicPrompt(
            action.text,
            lesson.topic,
          ),
          size: "1024x1024",
        }),
      });
      const payload = await response.json();
      const artifact = payload.artifact as InfographicArtifact | undefined;
      if (!artifact) {
        setInfographicStatus(
          payload.error ?? "The selected passage could not be illustrated yet.",
        );
        return;
      }

      setInfographicArtifact(artifact);
      setHasGeneratedInfographic(Boolean(artifact.generated));
      setSelectionInfographics((currentInfographics) => [
        ...currentInfographics,
        {
          artifact,
          id: `selection-${Date.now()}-${currentInfographics.length}`,
          pageIndex: action.pageIndex,
          sourceText: action.text,
          topic: lesson.topic,
        },
      ]);
      hydratePersistedBook(normalizeBookState(payload.book), authenticatedStudent);
      scheduleBookLayoutRefresh(action.pageIndex);
      setSelectedTextAction(null);
      window.getSelection()?.removeAllRanges();
      setInfographicStatus(
        artifact.generated
          ? `Selected passage illustrated with ${
              artifact.model ?? "gpt-image-2"
            }.`
          : (artifact.message ??
              "Set OPENAI_API_KEY in backend/.env to generate an image."),
      );
    } catch (error) {
      setInfographicStatus(
        `Could not generate an infographic for the selection: ${String(error)}`,
      );
    } finally {
      setIsSelectionInfographicLoading(false);
    }
  }

  function clearNarrationAudio() {
    const audio = narrationAudioRef.current;
    if (!audio) {
      setIsInfographicExplanationPlaying(false);
      return;
    }

    audio.pause();
    audio.src = "";
    narrationAudioRef.current = null;
    setIsInfographicExplanationPlaying(false);
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

  async function playInfographicExplanationAudio(
    explanation: InfographicExplanationPayload,
    options?: { fromSavedFile?: boolean },
  ) {
    const audioDataUrl = explanation.speech?.audioDataUrl;
    if (!explanation.speechGenerated || !audioDataUrl) {
      setInfographicExplanationStatus(
        explanation.speech?.message ??
          explanation.speech?.error ??
          "GPT-5.5 explained the diagram, but TTS was unavailable.",
      );
      return false;
    }

    const voice = explanation.voice ?? explanation.speech?.voice ?? "fable";
    const audio = new Audio(audioDataUrl);
    narrationAudioRef.current = audio;
    audio.onended = () => {
      narrationAudioRef.current = null;
      setIsInfographicExplanationPlaying(false);
      const persisted = explanation.persistedVoiceover;
      const replayNote =
        options?.fromSavedFile || explanation.cached || persisted?.reused
          ? " Replayed from the saved backend file."
          : persisted?.saved
            ? " Saved on the backend for next time."
            : persisted?.error
              ? " Backend storage could not save it."
              : "";
      setInfographicExplanationStatus(
        `Diagram explained with ${explanation.model ?? "GPT-5.5"} and ${voice} voice.${replayNote}`,
      );
    };
    audio.onerror = () => {
      narrationAudioRef.current = null;
      setIsInfographicExplanationPlaying(false);
      setInfographicExplanationStatus("Could not play the diagram narration.");
    };

    setIsNarrating(false);
    setIsInfographicExplanationPlaying(true);
    setInfographicExplanationStatus(
      `Playing ${
        options?.fromSavedFile || explanation.cached ? "saved " : ""
      }diagram explanation with ${voice} voice.`,
    );

    try {
      await audio.play();
      return true;
    } catch (error) {
      clearNarrationAudio();
      setIsNarrating(false);
      setInfographicExplanationStatus(
        `Could not play the diagram narration: ${String(error)}`,
      );
      return false;
    }
  }

  async function explainEnlargedInfographic() {
    if (!authenticatedStudent || !enlargedInfographic) {
      return;
    }

    if (isInfographicExplanationLoading) {
      return;
    }

    if (isInfographicExplanationPlaying || narrationAudioRef.current) {
      clearNarrationAudio();
      setIsNarrating(false);
      setInfographicExplanationStatus("Diagram narration stopped.");
      return;
    }

    if (infographicExplanation?.speech?.audioDataUrl) {
      await playInfographicExplanationAudio(infographicExplanation, {
        fromSavedFile: Boolean(
          infographicExplanation.cached ||
            infographicExplanation.persistedVoiceover?.saved ||
            infographicExplanation.persistedVoiceover?.reused,
        ),
      });
      return;
    }

    setIsInfographicExplanationLoading(true);
    setInfographicExplanation(null);
    setInfographicExplanationStatus("Reading the diagram with GPT-5.5 vision...");
    const requestId = infographicExplanationRequestIdRef.current + 1;
    infographicExplanationRequestIdRef.current = requestId;

    try {
      const response = await fetch(
        `${apiBaseUrl}/api/artifact/infographic/explain`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json", ...authHeaders(session) },
          body: JSON.stringify({
            studentId: authenticatedStudent.studentId,
            topic: enlargedInfographic.title ?? lesson.topic,
            imageSrc: enlargedInfographic.src,
            title: enlargedInfographic.title,
            alt: enlargedInfographic.alt,
            prompt: lesson.infographicPrompt,
          }),
        },
      );
      const payload = await response.json();
      const explanation = payload.explanation as
        | InfographicExplanationPayload
        | undefined;

      if (infographicExplanationRequestIdRef.current !== requestId) {
        return;
      }

      if (!explanation?.generated) {
        setInfographicExplanation(explanation ?? null);
        setInfographicExplanationStatus(
          explanation?.message ??
            explanation?.error ??
            payload.error ??
            "The diagram explanation could not be generated yet.",
        );
        return;
      }

      setInfographicExplanation(explanation);
      await playInfographicExplanationAudio(explanation, {
        fromSavedFile: Boolean(
          explanation.cached || explanation.persistedVoiceover?.reused,
        ),
      });
    } catch (error) {
      if (infographicExplanationRequestIdRef.current !== requestId) {
        return;
      }
      clearNarrationAudio();
      setIsNarrating(false);
      setInfographicExplanationStatus(
        `Could not explain this diagram: ${String(error)}`,
      );
    } finally {
      if (infographicExplanationRequestIdRef.current === requestId) {
        setIsInfographicExplanationLoading(false);
      }
    }
  }

  async function startTopic(
    nextTopic = topic,
    options?: { keepStagegateVisibleUntilLoaded?: boolean },
  ) {
    if (!authenticatedStudent) {
      return;
    }

    const cleanTopic = nextTopic.trim();
    const keepStagegateVisible =
      options?.keepStagegateVisibleUntilLoaded ?? false;

    stopNarration("OpenAI narration is ready.");
    infographicExplanationRequestIdRef.current += 1;
    setTopic(cleanTopic);
    setLessonStatus("Asking OpenAI Responses to guide this path...");
    setIsLessonGenerating(true);
    setHasAsked(true);
    setHasGeneratedInfographic(false);
    setIsInfographicGenerating(false);
    setInfographicArtifact(null);
    setSelectionInfographics([]);
    setSelectedTextAction(null);
    setInfographicExplanation(null);
    setIsInfographicExplanationLoading(false);
    setIsInfographicExplanationPlaying(false);
    setInfographicExplanationStatus("Ready to explain the enlarged diagram.");
    setIsStagegateSubmitting(false);
    if (!keepStagegateVisible) {
      setHasPassedStagegate(false);
      setStagegateResult(emptyStagegateResult);
      setAnswer("");
    }

    try {
      const payload = await requestLessonStart(
        authenticatedStudent,
        session,
        cleanTopic || undefined,
      );
      const responseStudent =
        syncAuthenticatedStudentSnapshot(payload.student) ?? authenticatedStudent;
      if (payload.error || !payload.lesson) {
        setRemoteMemories(normalizeMemories(responseStudent.memories));
        setLessonStatus(
          payload.error ?? "This lesson could not be generated yet.",
        );
        if (isMemoryOpen) {
          void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
        }
        return;
      }

      const normalizedLesson = normalizeLesson(
        payload.lesson,
        cleanTopic || firstTopicHint(responseStudent),
      );
      setHasPassedStagegate(false);
      setStagegateResult(emptyStagegateResult);
      setAnswer("");
      setLesson(normalizedLesson);
      setTopic(normalizedLesson.topic);
      setRemoteMemories(normalizeMemories(responseStudent.memories));
      const updatedBook = normalizeBookState(payload.book);
      hydratePersistedBook(updatedBook, responseStudent);
      setLessonStatus(
        normalizedLesson.aiMode === "openai_responses"
          ? "Guided by OpenAI Responses."
          : "Guided by the student profile.",
      );
      void loadReportCard({ learner: responseStudent });
      if (isMemoryOpen) {
        void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
      }
      scheduleBookLayoutRefresh(
        updatedBook ? restoredBookTargetPage(updatedBook) : currentStoryPageIndex,
      );
    } catch (error) {
      setLessonStatus(`Could not reach backend: ${String(error)}`);
    } finally {
      setIsLessonGenerating(false);
    }
  }

  async function generateInfographic() {
    if (
      !authenticatedStudent ||
      isInfographicGenerating ||
      isSelectionInfographicLoading
    ) {
      return;
    }

    setIsInfographicGenerating(true);
    setHasGeneratedInfographic(false);
    setInfographicArtifact(null);
    setInfographicStatus("Generating infographic...");

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
      hydratePersistedBook(normalizeBookState(payload.book), authenticatedStudent);
      scheduleBookLayoutRefresh();
      setInfographicStatus(
        payload.artifact?.generated
          ? `Generated with ${payload.artifact.model ?? "gpt-image-2"}.`
          : (payload.artifact?.message ??
              "Set OPENAI_API_KEY in backend/.env to generate an image."),
      );
    } catch (error) {
      setInfographicStatus(`Could not reach backend: ${String(error)}`);
    } finally {
      setIsInfographicGenerating(false);
    }
  }

  async function submitStagegate(nextAnswer = answer) {
    if (!authenticatedStudent || isStagegateSubmitting) {
      return;
    }

    setAnswer(nextAnswer);
    setIsStagegateSubmitting(true);
    let shouldTurnToUnlock = false;

    try {
      const response = await fetch(`${apiBaseUrl}/api/tutor/stagegate`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders(session) },
        body: JSON.stringify({
          studentId: authenticatedStudent.studentId,
          topic: lesson.topic,
          answer: nextAnswer,
          stageLevel: lesson.stageLevel,
          stagegatePrompt: lesson.stagegatePrompt,
          checkForUnderstanding: lesson.checkForUnderstanding,
        }),
      });
      const payload = await response.json();
      const responseStudent =
        syncAuthenticatedStudentSnapshot(payload.student) ?? authenticatedStudent;
      if (payload.error || !payload.result) {
        setRemoteMemories(normalizeMemories(responseStudent.memories));
        hydratePersistedBook(normalizeBookState(payload.book), responseStudent);
        void loadReportCard({ learner: responseStudent });
        setHasPassedStagegate(false);
        setStagegateResult({
          ...emptyStagegateResult,
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
            payload.error ?? "The answer could not be graded yet.",
        });
        if (isMemoryOpen) {
          void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
        }
        return;
      }

      const normalizedResult = normalizeStagegateResult(payload.result);
      setStagegateResult(normalizedResult);
      setRemoteMemories(normalizeMemories(responseStudent.memories));
      setHasPassedStagegate(normalizedResult.passed);
      hydratePersistedBook(normalizeBookState(payload.book), responseStudent);
      void loadReportCard({ learner: responseStudent });
      shouldTurnToUnlock = normalizedResult.passed;
      if (isMemoryOpen) {
        void loadMemoryGraph(selectedMemoryNodeId ?? undefined);
      }
    } catch (error) {
      setHasPassedStagegate(false);
      setStagegateResult({
        ...emptyStagegateResult,
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
    } finally {
      setIsStagegateSubmitting(false);
    }

    if (shouldTurnToUnlock) {
      window.setTimeout(flipNext, 250);
    }
  }

  async function handleResetStudentRecord() {
    if (!authenticatedStudent || isResettingStudent) {
      return;
    }

    const learnerToReset = authenticatedStudent;
    const currentSession = session;
    setIsResettingStudent(true);
    setResetError(null);
    stopNarration("OpenAI narration is ready.");
    infographicExplanationRequestIdRef.current += 1;
    setLessonStatus("Resetting this student record...");

    try {
      const resetPayload = await resetStudentRecord(
        learnerToReset,
        currentSession,
      );
      if (resetPayload.error || !resetPayload.student) {
        const message =
          resetPayload.error ?? "The student record could not be reset.";
        setResetError(message);
        setLessonStatus(message);
        return;
      }

      const resetStudent = syncAuthenticatedStudentSnapshot(
        resetPayload.student,
        currentSession,
      );
      if (!resetStudent) {
        const message = "The reset response did not include a valid student.";
        setResetError(message);
        setLessonStatus(message);
        return;
      }
      bootstrappedStudentIdRef.current = resetStudent.studentId;
      pendingBookEndPageRef.current = null;
      setSession(currentSession);
      setRemoteMemories(normalizeMemories(resetStudent.memories));
      setMemoryGraph(null);
      setIsMemoryOpen(false);
      setMemoryGraphStatus("Memory graph not loaded.");
      setSelectedMemoryNodeId(null);
      setCurrentPage(0);
      setTopic("");
      setLesson(initialLesson);
      setPersistedBook(null);
      setReportCard(null);
      setReportCardStatus("Report card not loaded.");
      setIsReportOpen(false);
      setHasAsked(true);
      setHasGeneratedInfographic(false);
      setHasPassedStagegate(false);
      setIsStagegateSubmitting(false);
      setIsLessonGenerating(false);
      setIsInfographicGenerating(false);
      setInfographicArtifact(null);
      setSelectionInfographics([]);
      setSelectedTextAction(null);
      setIsSelectionInfographicLoading(false);
      setEnlargedInfographic(null);
      setInfographicExplanation(null);
      setIsInfographicExplanationLoading(false);
      setIsInfographicExplanationPlaying(false);
      setInfographicExplanationStatus("Ready to explain the enlarged diagram.");
      setStagegateResult(emptyStagegateResult);
      setAnswer("");
      setInfographicStatus("No generated infographic yet.");
      setLessonStatus(
        "Student record reset. Generating a fresh opening path...",
      );
      setIsLessonGenerating(true);
      setIsResetDialogOpen(false);
      goToPage(0);

      const lessonPayload = await requestLessonStart(
        resetStudent,
        currentSession,
      );
      const lessonStudent =
        syncAuthenticatedStudentSnapshot(lessonPayload.student, currentSession) ??
        resetStudent;
      setRemoteMemories(normalizeMemories(lessonStudent.memories));
      if (lessonPayload.error || !lessonPayload.lesson) {
        setLessonStatus(
          lessonPayload.error ??
            "The fresh opening lesson could not be generated yet.",
        );
        return;
      }

      const normalizedLesson = normalizeLesson(
        lessonPayload.lesson,
        firstTopicHint(lessonStudent),
      );
      setLesson(normalizedLesson);
      setTopic(normalizedLesson.topic);
      hydratePersistedBook(
        normalizeBookState(lessonPayload.book),
        lessonStudent,
        {
          turnToEnd: true,
        },
      );
      setLessonStatus(
        normalizedLesson.aiMode === "openai_responses"
          ? "Fresh opening path generated by OpenAI Responses."
          : "Fresh opening path generated from the student profile.",
      );
      void loadReportCard({
        learner: lessonStudent,
        session: currentSession,
      });
    } catch (error) {
      const message = `Could not reset the student record: ${String(error)}`;
      setResetError(message);
      setLessonStatus(message);
    } finally {
      setIsResettingStudent(false);
      setIsLessonGenerating(false);
    }
  }

  function handleAuthenticated(payload: AuthPayload) {
    const nextSession = payload.session ?? null;
    const studentProfile = syncAuthenticatedStudentSnapshot(
      payload.student,
      nextSession,
    );
    if (!studentProfile) {
      return;
    }

    setSession(nextSession);
    setRemoteMemories(normalizeMemories(studentProfile.memories));
    if (studentProfile.suggestedTopics.length > 0) {
      setLesson((currentLesson) => ({
        ...currentLesson,
        suggestedTopics: studentProfile.suggestedTopics,
      }));
    }
    setTopic("");
  }

  function handleLogout() {
    clearStoredAuth();
    stopNarration("OpenAI narration is ready.");
    infographicExplanationRequestIdRef.current += 1;
    bootstrappedStudentIdRef.current = null;
    setAuthenticatedStudent(null);
    setSession(null);
    setRemoteMemories(null);
    setReportCard(null);
    setReportCardStatus("Report card not loaded.");
    setIsReportCardRefreshing(false);
    setIsReportOpen(false);
    setMemoryGraph(null);
    setIsMemoryOpen(false);
    setIsResetDialogOpen(false);
    setIsResettingStudent(false);
    setResetError(null);
    setMemoryGraphStatus("Memory graph not loaded.");
    setSelectedMemoryNodeId(null);
    setCurrentPage(0);
    setTopic("");
    setLesson(initialLesson);
    setPersistedBook(null);
    setHasAsked(false);
    setHasGeneratedInfographic(false);
    setHasPassedStagegate(false);
    setIsStagegateSubmitting(false);
    setIsLessonGenerating(false);
    setIsInfographicGenerating(false);
    setInfographicArtifact(null);
    setSelectionInfographics([]);
    setSelectedTextAction(null);
    setIsSelectionInfographicLoading(false);
    setEnlargedInfographic(null);
    setInfographicExplanation(null);
    setIsInfographicExplanationLoading(false);
    setIsInfographicExplanationPlaying(false);
    setInfographicExplanationStatus("Ready to explain the enlarged diagram.");
    setStagegateResult(emptyStagegateResult);
    setAnswer("");
    setLessonStatus("A starting point will be chosen from the student profile.");
    setInfographicStatus("No generated infographic yet.");
  }

  if (!authChecked) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-[#111515] text-stone-100">
        <div className="inline-flex items-center gap-2 text-sm uppercase text-cyan-50/70">
          <BookOpen className="h-4 w-4" />
          Opening Primer
        </div>
      </main>
    );
  }

  if (!authenticatedStudent) {
    return <AuthGate onAuthenticated={handleAuthenticated} />;
  }

  const learner = authenticatedStudent;
  const lessonGenerationTopicHint = lessonStatus
    .toLowerCase()
    .includes("opening path")
    ? openingProfileHint(learner)
    : topic || lesson.topic || firstTopicHint(learner);

  return (
    <main className="relative min-h-[100svh] overflow-hidden bg-[#111515] text-stone-100">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_10%,rgba(76,139,141,0.32),transparent_34%),radial-gradient(circle_at_82%_28%,rgba(215,170,92,0.18),transparent_28%),linear-gradient(135deg,#101818_0%,#182321_54%,#0d1112_100%)]" />
      <div className="absolute inset-x-0 top-0 h-px bg-cyan-100/20" />

      <section className="book-stage relative z-10 flex min-h-[100svh] w-full flex-col items-center justify-center gap-3 px-2 pb-3 pt-14 sm:px-5 sm:pb-5 sm:pt-16">
        <div className="absolute left-3 top-3 z-30 hidden items-center gap-2 rounded-full border border-cyan-100/15 bg-black/24 px-3 py-2 text-sm text-cyan-50 shadow-2xl shadow-black/20 sm:flex">
          <BookOpen className="h-4 w-4" />
          <span>{learner.displayName}&apos;s Primer</span>
        </div>
        <div className="absolute left-2 right-2 top-3 z-30 flex items-center justify-end gap-1.5 sm:left-auto sm:right-3 sm:gap-2">
          <XpBadge xpTotal={learner.xpTotal} />
          <button
            type="button"
            aria-label="Open report card"
            onClick={openReportCard}
            aria-busy={isReportCardRefreshing}
            className="inline-flex h-10 items-center gap-2 rounded-full border border-cyan-100/15 bg-black/28 px-2.5 text-sm text-cyan-50/78 shadow-2xl shadow-black/20 transition hover:text-cyan-50 sm:px-3"
          >
            <MapIcon className="h-4 w-4" />
            <span className="hidden sm:inline">Report</span>
          </button>
          <button
            type="button"
            aria-label="Open memory graph"
            onClick={openMemoryGraph}
            className="inline-flex h-10 items-center gap-2 rounded-full border border-cyan-100/15 bg-black/28 px-2.5 text-sm text-cyan-50/78 shadow-2xl shadow-black/20 transition hover:text-cyan-50 sm:px-3"
          >
            <Brain className="h-4 w-4" />
            <span className="hidden sm:inline">Memory</span>
          </button>
          <button
            type="button"
            aria-label="Reset student record"
            onClick={() => {
              setResetError(null);
              setIsResetDialogOpen(true);
            }}
            disabled={isResettingStudent}
            className="inline-flex h-10 items-center gap-2 rounded-full border border-amber-200/20 bg-black/28 px-2.5 text-sm text-amber-50/82 shadow-2xl shadow-black/20 transition hover:text-amber-50 disabled:cursor-wait disabled:opacity-60 sm:px-3"
          >
            <RotateCcw className="h-4 w-4" />
            <span className="hidden sm:inline">Reset</span>
          </button>
          <button
            type="button"
            aria-label="Sign out"
            onClick={handleLogout}
            className="inline-flex h-10 items-center gap-2 rounded-full border border-cyan-100/15 bg-black/28 px-2.5 text-sm text-cyan-50/78 shadow-2xl shadow-black/20 transition hover:text-cyan-50 sm:px-3"
          >
            <LogOut className="h-4 w-4" />
            <span className="hidden sm:inline">Sign out</span>
          </button>
        </div>

        <div
          ref={bookShellRef}
          className="primer-book-shell relative"
          onKeyUpCapture={scheduleSelectionCapture}
          onMouseUpCapture={scheduleSelectionCapture}
          onTouchEndCapture={scheduleSelectionCapture}
        >
          <div className="absolute inset-x-8 top-1/2 h-10 -translate-y-1/2 rounded-full bg-black/35 blur-3xl" />
          {isLessonGenerating ? (
            <LessonGenerationOverlay
              learnerName={learner.displayName}
              status={lessonStatus}
              topicHint={lessonGenerationTopicHint}
            />
          ) : null}
          <HTMLFlipBook
            key={flipBookStructureKey}
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
            disableFlipByClick
            onFlip={(event: PageFlipEvent<number>) =>
              setCurrentPage(Number(event.data))
            }
          >
                <BookPage
                  density="hard"
                  inlineInfographics={inlineInfographicsForPage(0)}
                  onLayoutChange={() => scheduleBookLayoutRefresh(0)}
                  onOpenInfographic={openInfographicImage}
                  pageIndex={0}
                  tone="cover"
                >
                  <CoverPage learner={learner} lesson={lesson} />
                </BookPage>

                <BookPage
                  inlineInfographics={inlineInfographicsForPage(1)}
                  onLayoutChange={() => scheduleBookLayoutRefresh(1)}
                  onOpenInfographic={openInfographicImage}
                  pageIndex={1}
                  pageNumber={1}
                >
                  <WelcomePage learner={learner} memories={visibleMemories} />
                </BookPage>

                <BookPage
                  inlineInfographics={inlineInfographicsForPage(2)}
                  onLayoutChange={() => scheduleBookLayoutRefresh(2)}
                  onOpenInfographic={openInfographicImage}
                  pageIndex={2}
                  pageNumber={2}
                >
                  <StageMapPage stages={visibleStages} />
                </BookPage>

                <BookPage
                  inlineInfographics={inlineInfographicsForPage(3)}
                  onLayoutChange={() => scheduleBookLayoutRefresh(3)}
                  onOpenInfographic={openInfographicImage}
                  pageIndex={3}
                  pageNumber={3}
                >
                  <AskPage
                    hasAsked={hasAsked}
                    lessonStatus={lessonStatus}
                    suggestedTopics={lesson.suggestedTopics}
                    topic={topic}
                    onAsk={(nextTopic) => void startTopic(nextTopic)}
                    onChooseTopic={(nextTopic) => void startTopic(nextTopic)}
                  />
                </BookPage>

                {displayedBookLessons.flatMap((entry, entryIndex) => {
                  const basePageIndex = lessonStoryPageIndex(entryIndex);
                  const infographicPageIndex = basePageIndex + 1;
                  const voiceoverPageIndex = basePageIndex + 2;
                  const followUpPageIndex = basePageIndex + 3;
                  const stagegatePageIndex = basePageIndex + 4;
                  const savedInfographic = infographicArtifactFromValue(
                    entry.bookLesson?.latestInfographic,
                  );
                  const entryInfographic = entry.isCurrent
                    ? infographicArtifact
                    : savedInfographic;
                  const entryStagegateResult = entry.isCurrent
                    ? stagegateResult
                    : normalizeStagegateResult(entry.bookLesson?.latestStagegate);
                  const entryAnswer = entry.isCurrent
                    ? answer
                    : (entry.bookLesson?.latestAnswer ?? "");
                  const entryHasPassed = entry.isCurrent
                    ? hasPassedStagegate
                    : entryStagegateResult.passed;
                  const entryInfographicStatus = entry.isCurrent
                    ? infographicStatus
                    : savedInfographicStatus(savedInfographic);
                  const pageNumberBase = basePageIndex;

                  return [
                    <BookPage
                      key={`${entry.lessonId}-story`}
                      inlineInfographics={inlineInfographicsForPage(
                        basePageIndex,
                      )}
                      onLayoutChange={() =>
                        scheduleBookLayoutRefresh(basePageIndex)
                      }
                      onOpenInfographic={openInfographicImage}
                      pageIndex={basePageIndex}
                      pageNumber={pageNumberBase}
                    >
                      <StoryPage lesson={entry.lesson} />
                    </BookPage>,
                    <BookPage
                      key={`${entry.lessonId}-infographic`}
                      inlineInfographics={inlineInfographicsForPage(
                        infographicPageIndex,
                      )}
                      onLayoutChange={() =>
                        scheduleBookLayoutRefresh(infographicPageIndex)
                      }
                      onOpenInfographic={openInfographicImage}
                      pageIndex={infographicPageIndex}
                      pageNumber={pageNumberBase + 1}
                    >
                      <InfographicPage
                        artifact={entryInfographic}
                        canGenerate={entry.isCurrent}
                        infographicStatus={entryInfographicStatus}
                        isGenerating={
                          entry.isCurrent &&
                          (isInfographicGenerating ||
                            isSelectionInfographicLoading)
                        }
                        lesson={entry.lesson}
                        hasGeneratedInfographic={
                          entry.isCurrent
                            ? hasGeneratedInfographic
                            : Boolean(savedInfographic?.generated)
                        }
                        onGenerate={
                          entry.isCurrent
                            ? () => void generateInfographic()
                            : undefined
                        }
                        onImageLoad={() =>
                          scheduleBookLayoutRefresh(infographicPageIndex)
                        }
                        onOpenInfographic={openInfographicImage}
                      />
                    </BookPage>,
                    <BookPage
                      key={`${entry.lessonId}-voiceover`}
                      inlineInfographics={inlineInfographicsForPage(
                        voiceoverPageIndex,
                      )}
                      onLayoutChange={() =>
                        scheduleBookLayoutRefresh(voiceoverPageIndex)
                      }
                      onOpenInfographic={openInfographicImage}
                      pageIndex={voiceoverPageIndex}
                      pageNumber={pageNumberBase + 2}
                    >
                      <VoiceoverPage
                        lesson={entry.lesson}
                        isNarrating={entry.isCurrent ? isNarrating : false}
                        isNarrationLoading={
                          entry.isCurrent ? isNarrationLoading : false
                        }
                        narrationStatus={
                          entry.isCurrent
                            ? narrationStatus
                            : "Saved lesson text is available for review."
                        }
                        onPlayNarration={
                          entry.isCurrent
                            ? () => void playNarration()
                            : undefined
                        }
                      />
                    </BookPage>,
                    <BookPage
                      key={`${entry.lessonId}-follow-up`}
                      inlineInfographics={inlineInfographicsForPage(
                        followUpPageIndex,
                      )}
                      onLayoutChange={() =>
                        scheduleBookLayoutRefresh(followUpPageIndex)
                      }
                      onOpenInfographic={openInfographicImage}
                      pageIndex={followUpPageIndex}
                      pageNumber={pageNumberBase + 3}
                    >
                      <FollowUpPage lesson={entry.lesson} />
                    </BookPage>,
                    <BookPage
                      key={`${entry.lessonId}-stagegate`}
                      inlineInfographics={inlineInfographicsForPage(
                        stagegatePageIndex,
                      )}
                      onLayoutChange={() =>
                        scheduleBookLayoutRefresh(stagegatePageIndex)
                      }
                      onOpenInfographic={openInfographicImage}
                      pageIndex={stagegatePageIndex}
                      pageNumber={pageNumberBase + 4}
                    >
                      {entry.isCurrent ? (
                        <StagegatePage
                          answer={entryAnswer}
                          hasPassed={entryHasPassed}
                          isSubmitting={isStagegateSubmitting}
                          lesson={entry.lesson}
                          onSubmit={(nextAnswer) =>
                            void submitStagegate(nextAnswer)
                          }
                          result={entryStagegateResult}
                        />
                      ) : (
                        <SavedStagegatePage
                          answer={entryAnswer}
                          lesson={entry.lesson}
                          result={entryStagegateResult}
                        />
                      )}
                    </BookPage>,
                  ];
                })}

                <BookPage
                  density="hard"
                  inlineInfographics={inlineInfographicsForPage(unlockPageIndex)}
                  onLayoutChange={() =>
                    scheduleBookLayoutRefresh(unlockPageIndex)
                  }
                  onOpenInfographic={openInfographicImage}
                  pageIndex={unlockPageIndex}
                  tone="deep"
                >
                  <UnlockPage
                    result={stagegateResult}
                    hasPassed={hasPassedStagegate}
                    lesson={lesson}
                    lessonStatus={lessonStatus}
                    onStartNextLesson={() => goToPage(nextTopicPageIndex)}
                  />
                </BookPage>

                <BookPage
                  inlineInfographics={inlineInfographicsForPage(
                    nextTopicPageIndex,
                  )}
                  onLayoutChange={() =>
                    scheduleBookLayoutRefresh(nextTopicPageIndex)
                  }
                  onOpenInfographic={openInfographicImage}
                  pageIndex={nextTopicPageIndex}
                  pageNumber={nextTopicPageIndex}
                >
                  {isNextTopicUnlocked ? (
                    <AskPage
                      hasAsked={hasAsked}
                      lessonStatus="Level complete. Choose the next thing to explore."
                      suggestedTopics={lesson.suggestedTopics}
                      topic={topic}
                      onAsk={(nextTopic) =>
                        void startTopic(nextTopic, {
                          keepStagegateVisibleUntilLoaded: true,
                        })
                      }
                      onChooseTopic={(nextTopic) =>
                        void startTopic(nextTopic, {
                          keepStagegateVisibleUntilLoaded: true,
                        })
                      }
                    />
                  ) : (
                    <LockedNextTopicPage
                      onReturnToStagegate={() =>
                        goToPage(currentStagegatePageIndex)
                      }
                    />
                  )}
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
        {selectedTextAction ? (
          <SelectedTextInfographicPopup
            action={selectedTextAction}
            isLoading={isSelectionInfographicLoading}
            onDismiss={() => {
              setSelectedTextAction(null);
              window.getSelection()?.removeAllRanges();
            }}
            onGenerate={() => void generateInfographicForSelection()}
          />
        ) : null}
      </section>
      {enlargedInfographic ? (
        <InfographicLightbox
          explanation={infographicExplanation}
          image={enlargedInfographic}
          isExplanationLoading={isInfographicExplanationLoading}
          isExplanationPlaying={isInfographicExplanationPlaying}
          status={infographicExplanationStatus}
          onClose={() => {
            infographicExplanationRequestIdRef.current += 1;
            if (isInfographicExplanationPlaying) {
              clearNarrationAudio();
            }
            setEnlargedInfographic(null);
            setInfographicExplanation(null);
            setIsInfographicExplanationLoading(false);
            setInfographicExplanationStatus("Ready to explain the enlarged diagram.");
          }}
          onExplain={() => void explainEnlargedInfographic()}
        />
      ) : null}
      {isResetDialogOpen ? (
        <ResetStudentDialog
          error={resetError}
          isResetting={isResettingStudent}
          learner={learner}
          onCancel={() => {
            if (!isResettingStudent) {
              setIsResetDialogOpen(false);
            }
          }}
          onConfirm={() => void handleResetStudentRecord()}
        />
      ) : null}
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
      {isReportOpen ? (
        <ReportCardDialog
          isRefreshing={isReportCardRefreshing}
          onClose={() => setIsReportOpen(false)}
          onRefresh={() => void loadReportCard()}
          reportCard={reportCard}
          status={reportCardStatus}
        />
      ) : null}
    </main>
  );
}

function XpBadge({ xpTotal }: { xpTotal: number }) {
  const safeXp = Number.isFinite(xpTotal)
    ? Math.max(0, Math.floor(xpTotal))
    : 0;

  return (
    <div className="inline-flex h-10 min-w-[82px] items-center justify-center gap-2 rounded-[8px] border border-[#d8b86a]/45 bg-[#d8b86a] px-3 text-[#17201d] shadow-2xl shadow-black/20">
      <Sparkles className="h-4 w-4" />
      <span className="text-xs font-semibold uppercase leading-none">XP</span>
      <span className="text-sm font-bold leading-none tabular-nums">{safeXp}</span>
    </div>
  );
}

function CoverPage({
  learner,
  lesson,
}: {
  learner: AuthenticatedStudent;
  lesson: PrimerLesson;
}) {
  return (
    <div className="flex h-full flex-col justify-between text-stone-50">
      <div>
        <div className="inline-flex items-center gap-2 border border-cyan-100/25 px-3 py-1 text-xs uppercase text-cyan-50/80">
          <Feather className="h-3.5 w-3.5" />
          Adaptive story tutor
        </div>
        <h2 className="mt-8 max-w-[12ch] text-5xl font-semibold leading-[1.02] sm:text-6xl">
          Primer
        </h2>
        <p className="mt-5 max-w-xs text-base leading-7 text-cyan-50/78">
          A guided lesson for {learner.displayName}, focused on {lesson.topic}.
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
      : "the profile details from signup";

  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Compass}>Welcome back</Kicker>
      <h2 className="mt-4 text-4xl font-semibold leading-tight text-stone-950">
        Welcome back, {learner.displayName}.
      </h2>
      <p className="mt-4 text-lg leading-8 text-stone-700">
        Your profile highlights these interests: {interestText}.
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
      <Kicker icon={MapIcon}>Stage map</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        Three levels guide this path.
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
        Passing Level 1 unlocks the mechanism view without losing the story
        thread.
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
}: {
  hasAsked: boolean;
  lessonStatus: string;
  suggestedTopics: string[];
  topic: string;
  onAsk: (topic: string) => void;
  onChooseTopic: (topic: string) => void;
}) {
  const topicInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    const input = topicInputRef.current;
    if (input && document.activeElement !== input) {
      input.value = topic;
    }
  }, [topic]);

  function currentTopic() {
    return topicInputRef.current?.value ?? topic;
  }

  function chooseTopic(nextTopic: string) {
    if (topicInputRef.current) {
      topicInputRef.current.value = nextTopic;
    }
    onChooseTopic(nextTopic);
  }

  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Mic}>Choose topic</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        Choose the next thing to explore.
      </h2>
      <div className="mt-6 rounded-[8px] border border-stone-300 bg-white/62 p-4">
        <label
          {...pageFlipInteractiveHandlers}
          htmlFor="topic"
          className="text-xs uppercase text-stone-500"
        >
          Student topic
        </label>
        <input
          {...pageFlipInteractiveHandlers}
          ref={topicInputRef}
          id="topic"
          defaultValue={topic}
          className="mt-3 h-12 w-full rounded-[8px] border border-stone-300 bg-white px-4 text-base font-semibold text-stone-900 outline-none focus:border-[#1e6f73] focus:ring-2 focus:ring-[#1e6f73]/20"
          placeholder="Leave blank for a profile-based starting point"
        />
      </div>
      <div className="mt-5 grid gap-3">
        <button
          {...pageFlipInteractiveHandlers}
          type="button"
          onClick={() => onAsk(currentTopic())}
          className="inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#1e6f73] px-5 text-sm font-semibold text-white transition hover:bg-[#195e61]"
        >
          <Volume2 className="h-4 w-4" />
          {hasAsked ? "Guide this topic again" : "Guide this topic"}
        </button>
      </div>

      <p className="mt-4 text-sm leading-6 text-stone-600">{lessonStatus}</p>

      <div className="mt-6">
        <p className="text-xs uppercase text-stone-500">Platform guidance</p>
        <div className="mt-3 flex flex-wrap gap-2">
          {suggestedTopics.map((suggestedTopic) => (
            <button
              {...pageFlipInteractiveHandlers}
              key={suggestedTopic}
              type="button"
              onClick={() => chooseTopic(suggestedTopic)}
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

function LockedNextTopicPage({
  onReturnToStagegate,
}: {
  onReturnToStagegate: () => void;
}) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Lock}>Next path</Kicker>
      <h2 className="mt-4 text-3xl font-semibold leading-tight text-stone-950">
        Finish this level first.
      </h2>
      <p className="mt-5 text-base leading-7 text-stone-700">
        The next topic chooser opens after the stagegate records a passing
        result.
      </p>
      <div className="mt-6 rounded-[8px] border border-[#d8b86a]/55 bg-[#fff8df] p-4 text-[#514010]">
        <p className="text-xs uppercase">Stagegate required</p>
        <p className="mt-2 text-sm leading-6">
          Submit an answer on the stagegate page to unlock the next learning
          path.
        </p>
      </div>
      <button
        {...pageFlipInteractiveHandlers}
        type="button"
        onClick={onReturnToStagegate}
        className="mt-auto inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#1e6f73] px-5 text-sm font-semibold text-white transition hover:bg-[#195e61]"
      >
        <ChevronLeft className="h-4 w-4" />
        Return to stagegate
      </button>
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
  canGenerate = true,
  infographicStatus,
  isGenerating,
  lesson,
  hasGeneratedInfographic,
  onGenerate,
  onImageLoad,
  onOpenInfographic,
}: {
  artifact: InfographicArtifact | null;
  canGenerate?: boolean;
  infographicStatus: string;
  isGenerating: boolean;
  lesson: PrimerLesson;
  hasGeneratedInfographic: boolean;
  onGenerate?: () => void;
  onImageLoad: () => void;
  onOpenInfographic: (image: EnlargedInfographic) => void;
}) {
  const imageSrc = artifactImageSrc(artifact);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-start justify-between gap-3">
        <Kicker icon={Sparkles}>
          {canGenerate ? "AI infographic tool" : "Saved infographic"}
        </Kicker>
        {canGenerate && onGenerate ? (
          <button
            {...pageFlipInteractiveHandlers}
            type="button"
            onClick={onGenerate}
            disabled={isGenerating}
            className="inline-flex h-10 min-w-[8rem] items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-4 text-xs font-semibold text-[#1c211d] transition hover:bg-[#e4c77a] disabled:cursor-wait disabled:opacity-75"
          >
            {isGenerating ? (
              <>
                <Sparkles className="h-3.5 w-3.5 animate-pulse" />
                Generating
              </>
            ) : hasGeneratedInfographic ? (
              "Regenerate"
            ) : (
              "Generate"
            )}
          </button>
        ) : null}
      </div>
      <h2 className="mt-3 text-2xl font-semibold text-stone-950">
        {lesson.topic} infographic
      </h2>
      <p className="mt-1 text-sm text-stone-600">{infographicStatus}</p>
      {imageSrc ? (
        <InfographicImageButton
          alt={`Generated infographic for ${lesson.topic}`}
          className="mt-4 block aspect-square w-full overflow-hidden rounded-[8px] border border-stone-300 bg-white"
          imageClassName="h-full w-full object-cover"
          onImageLoad={onImageLoad}
          onOpen={onOpenInfographic}
          src={imageSrc}
          title={`${lesson.topic} infographic`}
        />
      ) : (
        <InfographicFrame isGenerating={isGenerating} />
      )}
    </div>
  );
}

function InfographicFrame({ isGenerating }: { isGenerating: boolean }) {
  return (
    <div
      aria-label={
        isGenerating
          ? "Generating infographic"
          : "Generated infographic will appear here"
      }
      aria-live="polite"
      role={isGenerating ? "status" : undefined}
      className="infographic-empty-frame mt-4 flex aspect-square w-full items-center justify-center overflow-hidden rounded-[8px] border border-stone-300 bg-white/55"
    >
      {isGenerating ? (
        <div className="infographic-spark-field" aria-hidden="true">
          {infographicSparkles.map((sparkle) => (
            <span
              key={`${sparkle.x}-${sparkle.y}`}
              className="infographic-spark-dot"
              style={{
                animationDelay: sparkle.delay,
                height: sparkle.size,
                left: sparkle.x,
                top: sparkle.y,
                width: sparkle.size,
              }}
            />
          ))}
        </div>
      ) : (
        <span className="sr-only">Generated infographic will appear here.</span>
      )}
    </div>
  );
}

function VoiceoverPage({
  lesson,
  isNarrating = false,
  isNarrationLoading = false,
  narrationStatus = "Saved lesson text is available for review.",
  onPlayNarration,
}: {
  lesson: PrimerLesson;
  isNarrating?: boolean;
  isNarrationLoading?: boolean;
  narrationStatus?: string;
  onPlayNarration?: () => void;
}) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Volume2}>Voice-over</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        Listen for the main idea in {lesson.topic}.
      </h2>
      <p className="mt-4 text-base leading-7 text-stone-700">
        {lesson.plainExplanation}
      </p>
      {onPlayNarration ? (
        <button
          {...pageFlipInteractiveHandlers}
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
      ) : null}
      <p className="mt-3 text-xs leading-5 text-stone-500">
        {onPlayNarration
          ? `${narrationStatus} This voice is AI-generated.`
          : narrationStatus}
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
        Connect {lesson.topic} to a familiar pattern.
      </h2>
      <p className="mt-5 text-lg leading-8 text-stone-800">
        {lesson.analogy}
      </p>
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
    </div>
  );
}

function StagegatePage({
  answer,
  hasPassed,
  isSubmitting,
  lesson,
  onSubmit,
  result,
}: {
  answer: string;
  hasPassed: boolean;
  isSubmitting: boolean;
  lesson: PrimerLesson;
  onSubmit: (answer: string) => void;
  result: StagegateResult;
}) {
  const answerInputRef = useRef<HTMLTextAreaElement | null>(null);
  const hasFeedback =
    !hasPassed &&
    !isSubmitting &&
    result.feedbackToStudent.trim() !== "" &&
    result.feedbackToStudent !== emptyStagegateResult.feedbackToStudent;

  useEffect(() => {
    const input = answerInputRef.current;
    if (input && document.activeElement !== input) {
      input.value = answer;
    }
  }, [answer]);

  function currentAnswer() {
    return answerInputRef.current?.value ?? answer;
  }

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
        {...pageFlipInteractiveHandlers}
        ref={answerInputRef}
        defaultValue={answer}
        disabled={isSubmitting}
        className={`mt-5 resize-none rounded-[8px] border border-stone-300 bg-white/70 p-4 text-sm leading-6 text-stone-800 outline-none transition focus:border-[#1e6f73] focus:ring-2 focus:ring-[#1e6f73]/20 disabled:cursor-wait disabled:opacity-70 ${
          hasFeedback || isSubmitting ? "min-h-[104px]" : "min-h-[150px]"
        }`}
      />
      {isSubmitting ? <StagegateThinkingPanel /> : null}
      {hasFeedback ? <StagegateGuidancePanel result={result} /> : null}
      <button
        {...pageFlipInteractiveHandlers}
        type="button"
        disabled={isSubmitting}
        onClick={() => onSubmit(currentAnswer())}
        className="mt-5 inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-5 text-sm font-semibold text-[#1c211d] transition hover:bg-[#e5c879] disabled:cursor-wait disabled:opacity-75"
      >
        {isSubmitting ? (
          <>
            <Cog className="h-4 w-4 animate-spin" />
            Thinking through your answer
          </>
        ) : hasPassed ? (
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

function SavedStagegatePage({
  answer,
  lesson,
  result,
}: {
  answer: string;
  lesson: PrimerLesson;
  result: StagegateResult;
}) {
  const hasAttempt = answer.trim() !== "";
  const hasFeedback =
    result.feedbackToStudent.trim() !== "" &&
    result.feedbackToStudent !== emptyStagegateResult.feedbackToStudent;

  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Check}>Saved stagegate</Kicker>
      <h2 className="mt-4 text-3xl font-semibold text-stone-950">
        {result.passed ? "Level complete." : "Saved attempt."}
      </h2>
      <p className="mt-4 text-sm leading-6 text-stone-600">
        {lesson.stagegatePrompt}
      </p>
      <div className="mt-5 rounded-[8px] border border-stone-300 bg-white/64 p-4">
        <p className="text-xs font-semibold uppercase text-stone-500">
          Student answer
        </p>
        <p className="mt-2 text-sm leading-6 text-stone-800">
          {hasAttempt ? answer : "No saved answer is attached to this lesson."}
        </p>
      </div>
      <div
        className={`mt-4 rounded-[8px] border p-4 ${
          result.passed
            ? "border-[#1e6f73]/30 bg-[#eaf8f6]/78 text-[#174f52]"
            : "border-amber-300/70 bg-amber-50/78 text-[#514010]"
        }`}
      >
        <p className="text-xs font-semibold uppercase">
          {result.passed
            ? `Passed - score ${Math.round(result.score * 100)}%`
            : `Not passed - score ${Math.round(result.score * 100)}%`}
        </p>
        <p className="mt-2 text-sm leading-6">
          {hasFeedback
            ? result.feedbackToStudent
            : "No assessor feedback was saved for this attempt."}
        </p>
      </div>
    </div>
  );
}

function StagegateThinkingPanel() {
  return (
    <div
      aria-live="polite"
      role="status"
      className="mt-4 flex items-center gap-3 rounded-[8px] border border-[#1e6f73]/25 bg-[#eaf8f6]/75 p-3 text-[#174f52]"
    >
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-white/78">
        <Brain className="h-5 w-5 animate-pulse" />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-xs font-semibold uppercase">
          Primer is checking the gate
        </p>
        <p className="mt-1 text-sm leading-5">
          Reading your reasoning, matching it to the rubric, and preparing the
          next page.
        </p>
      </div>
      <span className="flex shrink-0 items-center gap-1" aria-hidden="true">
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-[#1e6f73] [animation-delay:-0.2s]" />
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-[#1e6f73] [animation-delay:-0.1s]" />
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-[#1e6f73]" />
      </span>
    </div>
  );
}

function StagegateGuidancePanel({ result }: { result: StagegateResult }) {
  const guidance = result.gaps.slice(0, 3);

  return (
    <div
      aria-live="polite"
      className="mt-4 max-h-36 overflow-y-auto rounded-[8px] border border-amber-300/70 bg-amber-50/78 p-3 text-[#514010]"
    >
      <div className="flex items-start gap-2">
        <TriangleAlert className="mt-0.5 h-4 w-4 shrink-0" />
        <div>
          <p className="text-xs font-semibold uppercase">
            Try again - score {Math.round(result.score * 100)}%
          </p>
          <p className="mt-1 text-sm leading-5">{result.feedbackToStudent}</p>
        </div>
      </div>
      {guidance.length > 0 ? (
        <ul className="mt-2 space-y-1 pl-6 text-xs leading-5">
          {guidance.map((gap, index) => (
            <li key={`${index}-${gap}`} className="list-disc">
              {gap}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function UnlockPage({
  result,
  hasPassed,
  lesson,
  lessonStatus,
  onStartNextLesson,
}: {
  result: StagegateResult;
  hasPassed: boolean;
  lesson: PrimerLesson;
  lessonStatus: string;
  onStartNextLesson: () => void;
}) {
  const unlockedLabel =
    result.nextLevelUnlocked === "transfer"
      ? "Level 3: Transfer"
      : result.nextLevelUnlocked === "complete"
        ? "next path"
        : "Level 2: Mechanism";
  const hasAttemptFeedback =
    result.feedbackToStudent.trim() !== "" &&
    result.feedbackToStudent !== emptyStagegateResult.feedbackToStudent;

  return (
    <div className="flex h-full flex-col text-stone-50">
      <Kicker icon={Check} dark>
        Unlock result
      </Kicker>
      <h2 className="mt-5 text-4xl font-semibold leading-tight">
        {hasPassed ? "The next level opens." : "The next level is waiting."}
      </h2>
      <p className="mt-4 text-base leading-7 text-cyan-50/78">
        {hasPassed
          ? result.feedbackToStudent
          : hasAttemptFeedback
            ? result.feedbackToStudent
            : "Submit the stagegate page to unlock the next level and add a new visible memory."}
      </p>

      <div className="mt-6 space-y-3">
        {Object.entries(result.rubric).map(([label, value]) => (
          <RubricBar
            key={label}
            label={label}
            value={hasPassed || hasAttemptFeedback ? value : 0}
          />
        ))}
      </div>

      <div className="mt-6 rounded-[8px] border border-cyan-100/20 bg-cyan-50/10 p-4">
        <p className="text-xs uppercase text-cyan-100/75">New memory</p>
        <p className="mt-2 text-sm leading-6">
          {hasPassed
            ? (result.newMemories?.[0]?.content ??
              "This learning step was recorded in memory.")
            : "No new mastery memory has been added yet."}
        </p>
      </div>

      <div className="mt-auto rounded-[8px] bg-[#d8b86a] p-4 text-[#1c211d]">
        <p className="text-xs uppercase">Unlocked</p>
        <p className="mt-1 text-lg font-semibold">
          {hasPassed ? unlockedLabel : "Pass Level 1 first"}
        </p>
        {hasPassed ? (
          <>
            <button
              {...pageFlipInteractiveHandlers}
              type="button"
              onClick={onStartNextLesson}
              className="mt-3 inline-flex h-10 w-full items-center justify-center gap-2 rounded-full bg-[#173b3b] px-4 text-sm font-semibold text-cyan-50 transition hover:bg-[#1f4f4f]"
            >
              <Play className="h-4 w-4" />
              Choose next topic
            </button>
            <p
              aria-live="polite"
              className="mt-2 text-xs leading-5 text-[#3d3210]"
            >
              Continue {lesson.topic}. {lessonStatus}
            </p>
          </>
        ) : null}
      </div>
    </div>
  );
}

function InlineSelectionInfographics({
  infographics,
  onLayoutChange,
  onOpenInfographic,
}: {
  infographics: SelectionInfographic[];
  onLayoutChange?: () => void;
  onOpenInfographic?: (image: EnlargedInfographic) => void;
}) {
  return (
    <div className="mt-3 flex shrink-0 gap-2 overflow-x-auto border-t border-stone-300/70 pt-3">
      {infographics.map((infographic) => {
        const imageSrc = artifactImageSrc(infographic.artifact);
        const title = `Selection from ${infographic.topic}`;

        return (
          <div
            key={infographic.id}
            className="flex min-w-[190px] max-w-[230px] items-center gap-2 rounded-[8px] border border-stone-300 bg-white/68 p-2"
          >
            {imageSrc && onOpenInfographic ? (
              <InfographicImageButton
                alt={title}
                className="h-16 w-16 shrink-0 overflow-hidden rounded-[6px] border border-stone-300 bg-white"
                imageClassName="h-full w-full object-cover"
                onImageLoad={onLayoutChange}
                onOpen={onOpenInfographic}
                src={imageSrc}
                title={title}
              />
            ) : (
              <div className="flex h-16 w-16 shrink-0 items-center justify-center rounded-[6px] border border-stone-300 bg-[#173b3b] text-cyan-50">
                <Sparkles className="h-5 w-5" />
              </div>
            )}
            <div className="min-w-0">
              <p className="text-[10px] font-semibold uppercase text-[#1e6f73]">
                Selection diagram
              </p>
              <p className="mt-1 line-clamp-2 text-xs leading-4 text-stone-700">
                {infographic.sourceText}
              </p>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function InfographicImageButton({
  alt,
  className,
  imageClassName,
  onImageLoad,
  onOpen,
  src,
  title,
}: {
  alt: string;
  className: string;
  imageClassName: string;
  onImageLoad?: () => void;
  onOpen: (image: EnlargedInfographic) => void;
  src: string;
  title?: string;
}) {
  return (
    <button
      {...pageFlipInteractiveHandlers}
      type="button"
      aria-label="Enlarge infographic"
      title="Enlarge infographic"
      onClick={() => onOpen({ alt, src, title })}
      className={`group relative cursor-zoom-in ${className}`}
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={src}
        alt={alt}
        className={imageClassName}
        onLoad={onImageLoad}
      />
      <span className="absolute right-2 top-2 inline-flex h-7 w-7 items-center justify-center rounded-full bg-black/55 text-white opacity-0 transition group-hover:opacity-100 group-focus-visible:opacity-100">
        <Maximize2 className="h-3.5 w-3.5" />
      </span>
    </button>
  );
}

function SelectedTextInfographicPopup({
  action,
  isLoading,
  onDismiss,
  onGenerate,
}: {
  action: SelectedTextAction;
  isLoading: boolean;
  onDismiss: () => void;
  onGenerate: () => void;
}) {
  return (
    <div
      className="fixed z-40 w-[228px] rounded-[8px] border border-cyan-100/18 bg-[#0b1515]/96 p-2.5 text-cyan-50 shadow-2xl shadow-black/35 backdrop-blur"
      style={{ left: action.x, top: action.y }}
      onMouseDown={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
      onTouchStart={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    >
      <p className="line-clamp-2 text-xs leading-4 text-cyan-50/76">
        {action.text}
      </p>
      <div className="mt-2 flex items-center gap-2">
        <button
          type="button"
          disabled={isLoading}
          onClick={onGenerate}
          className="inline-flex h-9 flex-1 items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-3 text-xs font-semibold text-[#19221f] transition hover:bg-[#e4c77a] disabled:cursor-wait disabled:opacity-70"
        >
          <Sparkles className="h-3.5 w-3.5" />
          {isLoading ? "Generating" : "Infographic"}
        </button>
        <button
          type="button"
          aria-label="Dismiss selected text action"
          onClick={onDismiss}
          className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-cyan-100/18 text-cyan-50/76 transition hover:text-cyan-50"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}

function InfographicLightbox({
  explanation,
  image,
  isExplanationLoading,
  isExplanationPlaying,
  onClose,
  onExplain,
  status,
}: {
  explanation: InfographicExplanationPayload | null;
  image: EnlargedInfographic;
  isExplanationLoading: boolean;
  isExplanationPlaying: boolean;
  onClose: () => void;
  onExplain: () => void;
  status: string;
}) {
  const observations = explanation?.keyObservations ?? [];
  const hasSavedVoiceover = Boolean(
    explanation?.speech?.audioDataUrl &&
      (explanation.cached ||
        explanation.persistedVoiceover?.saved ||
        explanation.persistedVoiceover?.reused),
  );

  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-[#071111]/94 p-4 text-cyan-50 backdrop-blur-md"
      onClick={onClose}
    >
      <div
        className="relative grid max-h-full w-full max-w-6xl gap-3 lg:grid-cols-[minmax(0,1fr)_340px]"
        onClick={(event) => event.stopPropagation()}
      >
        <button
          type="button"
          aria-label="Close enlarged infographic"
          onClick={onClose}
          className="absolute right-2 top-2 z-10 inline-flex h-10 w-10 items-center justify-center rounded-full bg-black/60 text-cyan-50 transition hover:bg-black/75"
        >
          <X className="h-4 w-4" />
        </button>
        <div className="overflow-hidden rounded-[8px] border border-cyan-100/18 bg-black/24">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={image.src}
            alt={image.alt}
            className="max-h-[86svh] w-full object-contain"
          />
        </div>
        <aside className="max-h-[86svh] overflow-y-auto rounded-[8px] border border-cyan-100/18 bg-[#101919]/96 p-4 shadow-2xl shadow-black/30">
          <p className="text-xs uppercase text-cyan-100/70">
            Diagram voice explanation
          </p>
          <h2 className="mt-2 text-xl font-semibold leading-tight text-stone-50">
            {image.title ?? "Generated infographic"}
          </h2>
          <p className="mt-3 text-sm leading-6 text-stone-300">{status}</p>
          {isExplanationLoading ? <IndeterminateProgress /> : null}
          <button
            type="button"
            onClick={onExplain}
            disabled={isExplanationLoading}
            className="mt-4 inline-flex h-11 w-full items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-4 text-sm font-semibold text-[#19221f] transition hover:bg-[#e4c77a] disabled:cursor-wait disabled:opacity-70"
          >
            {isExplanationLoading ? (
              <>
                <Sparkles className="h-4 w-4" />
                Explaining diagram
              </>
            ) : isExplanationPlaying ? (
              <>
                <Square className="h-4 w-4" />
                Stop explanation
              </>
            ) : (
              <>
                <Volume2 className="h-4 w-4" />
                {hasSavedVoiceover ? "Replay saved audio" : "Explain aloud"}
              </>
            )}
          </button>
          <p className="mt-3 text-xs leading-5 text-cyan-50/58">
            {hasSavedVoiceover
              ? "Saved audio replays from the backend filesystem without regenerating."
              : "GPT-5.5 reads the image with vision input. The voice is AI-generated."}
          </p>

          {explanation?.explanation ? (
            <div className="mt-4 border-t border-cyan-100/12 pt-4">
              <p className="text-xs uppercase text-cyan-100/60">
                Explanation
              </p>
              <p className="mt-2 text-sm leading-6 text-stone-200">
                {explanation.explanation}
              </p>
            </div>
          ) : null}

          {observations.length > 0 ? (
            <div className="mt-4 space-y-2">
              {observations.map((observation) => (
                <div
                  key={observation}
                  className="rounded-[8px] border border-cyan-100/12 bg-cyan-50/10 px-3 py-2 text-xs leading-5 text-cyan-50/78"
                >
                  {observation}
                </div>
              ))}
            </div>
          ) : null}
        </aside>
      </div>
    </div>
  );
}

function IndeterminateProgress() {
  return (
    <div className="mt-4 h-2 overflow-hidden rounded-full bg-cyan-50/12">
      <div className="h-full w-1/2 animate-[pulse_1.2s_ease-in-out_infinite] rounded-full bg-[#d8b86a]" />
    </div>
  );
}

function ResetStudentDialog({
  error,
  isResetting,
  learner,
  onCancel,
  onConfirm,
}: {
  error: string | null;
  isResetting: boolean;
  learner: AuthenticatedStudent;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-[#071111]/92 p-4 text-cyan-50 backdrop-blur-md">
      <div className="w-full max-w-md rounded-[8px] border border-amber-200/24 bg-[#101919] p-5 shadow-2xl shadow-black/40">
        <div className="flex items-start gap-3">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-amber-300/16 text-amber-100">
            <TriangleAlert className="h-5 w-5" />
          </div>
          <div>
            <p className="text-xs uppercase text-amber-100/78">
              Reset student record
            </p>
            <h2 className="mt-2 text-2xl font-semibold text-stone-50">
              Start {learner.displayName}&apos;s Primer over?
            </h2>
            <p className="mt-3 text-sm leading-6 text-stone-300">
              This deletes this student&apos;s books, lesson pages, progress,
              story continuity, and memories. The signup profile and login stay
              in place so a fresh opening lesson can be generated immediately.
            </p>
          </div>
        </div>

        {error ? (
          <p className="mt-4 rounded-[8px] border border-red-300/25 bg-red-950/30 px-3 py-2 text-sm leading-6 text-red-100">
            {error}
          </p>
        ) : null}

        <div className="mt-6 flex flex-wrap items-center justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            disabled={isResetting}
            className="inline-flex h-10 items-center justify-center rounded-full border border-cyan-100/16 px-4 text-sm font-semibold text-cyan-50/78 transition hover:text-cyan-50 disabled:cursor-wait disabled:opacity-60"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={isResetting}
            className="inline-flex h-10 items-center justify-center gap-2 rounded-full bg-[#d8b86a] px-4 text-sm font-semibold text-[#19221f] transition hover:bg-[#e4c77a] disabled:cursor-wait disabled:opacity-70"
          >
            <RotateCcw className="h-4 w-4" />
            {isResetting ? "Resetting" : "Reset and restart"}
          </button>
        </div>
      </div>
    </div>
  );
}

function artifactImageSrc(artifact: InfographicArtifact | null | undefined) {
  return artifact?.imageDataUrl ?? artifact?.imageUrl ?? null;
}

function infographicArtifactFromValue(value: unknown): InfographicArtifact | null {
  return value && typeof value === "object"
    ? (value as InfographicArtifact)
    : null;
}

function savedInfographicStatus(artifact: InfographicArtifact | null): string {
  if (!artifact) {
    return "No infographic was saved for this lesson.";
  }

  if (artifact.generated) {
    return `Saved diagram generated with ${artifact.model ?? "gpt-image-2"}.`;
  }

  return (
    artifact.message ??
    artifact.error ??
    "A saved infographic fallback is attached to this lesson."
  );
}

function elementFromNode(node: Node | null): Element | null {
  if (!node) {
    return null;
  }

  if (node.nodeType === Node.ELEMENT_NODE) {
    return node as Element;
  }

  return node.parentElement;
}

function closestPrimerPage(node: Node | null): HTMLElement | null {
  return elementFromNode(node)?.closest<HTMLElement>(".primer-page") ?? null;
}

function pageIndexFromRange(range: Range): number | null {
  const page =
    closestPrimerPage(range.startContainer) ??
    closestPrimerPage(range.commonAncestorContainer);
  const index = Number(page?.dataset.primerPageIndex);

  return Number.isFinite(index) ? index : null;
}

function firstRangeBounds(range: Range): DOMRect | null {
  const bounds = range.getBoundingClientRect();
  if (bounds.width > 0 || bounds.height > 0) {
    return bounds;
  }

  for (const rect of Array.from(range.getClientRects())) {
    if (rect.width > 0 || rect.height > 0) {
      return rect;
    }
  }

  return null;
}

function popupPositionForBounds(bounds: DOMRect) {
  const popupWidth = 228;
  const x = clamp(
    bounds.left + bounds.width / 2 - popupWidth / 2,
    12,
    window.innerWidth - popupWidth - 12,
  );
  const yAbove = bounds.top - 68;
  const y = yAbove >= 12 ? yAbove : bounds.bottom + 10;

  return {
    x,
    y: clamp(y, 12, window.innerHeight - 96),
  };
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), Math.max(min, max));
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

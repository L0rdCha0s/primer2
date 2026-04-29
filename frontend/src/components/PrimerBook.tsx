"use client";

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
  Map,
  Mic,
  Play,
  Search,
  Sparkles,
  Square,
  Unlock,
  Volume2,
  Waves,
} from "lucide-react";
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
  themeBible,
  tutorScene,
  unlockedMemory,
} from "@/lib/demo-data";
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

type BackendMemory = {
  memory_type?: StudentMemory["type"];
  memoryType?: StudentMemory["type"];
  content?: string;
  confidence?: number;
  tags?: string[];
};

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
  map: Map,
  spark: Sparkles,
  water: Waves,
};

const pageTurnStyle: CSSProperties = {};
const studentId = "mina-demo";
const apiBaseUrl =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:4000";

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

export function PrimerBook() {
  const bookRef = useRef<FlipBookRef | null>(null);
  const [currentPage, setCurrentPage] = useState(0);
  const [topic, setTopic] = useState("lightning");
  const [lesson, setLesson] = useState<PrimerLesson>(initialLesson);
  const [remoteMemories, setRemoteMemories] = useState<StudentMemory[] | null>(
    null,
  );
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

  useEffect(() => {
    return () => {
      if ("speechSynthesis" in window) {
        window.speechSynthesis.cancel();
      }
    };
  }, []);

  const visibleMemories = useMemo<StudentMemory[]>(() => {
    const baseMemories = remoteMemories ?? memories;
    return hasPassedStagegate
      ? [
          ...baseMemories,
          {
            ...unlockedMemory,
            content: `Mina made progress on ${lesson.topic} at the ${lesson.stageLevel} level.`,
          },
        ]
      : baseMemories;
  }, [hasPassedStagegate, lesson.stageLevel, lesson.topic, remoteMemories]);

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

  function playNarration() {
    if (!("speechSynthesis" in window)) {
      return;
    }

    window.speechSynthesis.cancel();

    if (isNarrating) {
      setIsNarrating(false);
      return;
    }

    const utterance = new SpeechSynthesisUtterance(
      `${lesson.storyScene} ${lesson.plainExplanation} ${lesson.analogy}`,
    );
    utterance.rate = 0.93;
    utterance.pitch = 1.04;
    utterance.onend = () => setIsNarrating(false);
    utterance.onerror = () => setIsNarrating(false);
    setIsNarrating(true);
    window.speechSynthesis.speak(utterance);
  }

  async function startTopic(nextTopic = topic) {
    const cleanTopic = nextTopic.trim();
    if (!cleanTopic) {
      return;
    }

    setTopic(cleanTopic);
    setLessonStatus("Asking OpenAI Responses to guide this path...");
    setHasAsked(true);
    setHasGeneratedInfographic(false);
    setInfographicArtifact(null);
    setHasPassedStagegate(false);

    try {
      const response = await fetch(`${apiBaseUrl}/api/lesson/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          studentId,
          topic: cleanTopic,
          question: `I want to explore ${cleanTopic}. Guide me at my current level.`,
        }),
      });
      const payload = await response.json();
      setLesson(normalizeLesson(payload.lesson, cleanTopic));
      setRemoteMemories(normalizeMemories(payload.student?.memories));
      setLessonStatus(
        payload.lesson?.aiMode === "openai_responses"
          ? "Guided by OpenAI Responses."
          : "Using local fallback until backend/.env has OPENAI_API_KEY.",
      );
      goToPage(4);
    } catch (error) {
      setLessonStatus(`Could not reach backend: ${String(error)}`);
    }
  }

  async function generateInfographic() {
    setInfographicStatus("Calling gpt-image-2 for an infographic...");

    try {
      const response = await fetch(`${apiBaseUrl}/api/artifact/infographic`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          studentId,
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
    try {
      const response = await fetch(`${apiBaseUrl}/api/tutor/stagegate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          studentId,
          topic: lesson.topic,
          answer,
          stageLevel: lesson.stageLevel,
        }),
      });
      const payload = await response.json();
      setStagegateResult(normalizeStagegateResult(payload.result));
      setRemoteMemories(normalizeMemories(payload.student?.memories));
      setHasPassedStagegate(Boolean(payload.result?.passed));
    } catch {
      setHasPassedStagegate(true);
      setStagegateResult(seededStagegateResult);
    }

    window.setTimeout(flipNext, 250);
  }

  return (
    <main className="min-h-screen overflow-hidden bg-[#111515] text-stone-100">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_20%_10%,rgba(76,139,141,0.32),transparent_34%),radial-gradient(circle_at_82%_28%,rgba(215,170,92,0.18),transparent_28%),linear-gradient(135deg,#101818_0%,#182321_54%,#0d1112_100%)]" />
      <div className="absolute inset-x-0 top-0 h-px bg-cyan-100/20" />

      <div className="relative mx-auto flex min-h-screen w-full max-w-[1380px] flex-col px-4 py-4 sm:px-6 lg:px-8">
        <header className="flex flex-wrap items-center justify-between gap-4 py-3">
          <div>
            <p className="text-xs uppercase text-cyan-100/70">
              PrimerLab student view
            </p>
            <h1 className="mt-1 text-2xl font-semibold text-stone-50 sm:text-3xl">
              Mina&apos;s Clockwork Reef Primer
            </h1>
          </div>
          <div className="flex items-center gap-2 rounded-full border border-cyan-100/15 bg-cyan-50/10 px-3 py-2 text-sm text-cyan-50 shadow-2xl shadow-black/20">
            <BookOpen className="h-4 w-4" />
            <span>Real book mode</span>
          </div>
        </header>

        <div className="grid flex-1 gap-5 lg:grid-cols-[minmax(0,1fr)_300px]">
          <section className="book-stage flex min-h-[720px] items-center justify-center rounded-[8px] border border-cyan-100/10 bg-black/18 px-2 py-6 shadow-2xl shadow-black/40 sm:px-6">
            <div className="relative w-full max-w-[1100px]">
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
                maxWidth={500}
                minHeight={430}
                maxHeight={690}
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
                  <CoverPage />
                </BookPage>

                <BookPage pageNumber={1}>
                  <WelcomePage memories={visibleMemories} />
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
                    onPlayNarration={playNarration}
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

              <div className="mt-5 flex flex-wrap items-center justify-center gap-3">
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

          <aside className="flex flex-col gap-4 rounded-[8px] border border-cyan-100/10 bg-[#0d1414]/76 p-4 shadow-2xl shadow-black/30">
            <ProgressPanel stages={visibleStages} />
            <MemoryPanel memories={visibleMemories} />
            <ContinuityPanel />
          </aside>
        </div>
      </div>
    </main>
  );
}

function CoverPage() {
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
          A living lesson book for Mina, where charged pearls, storm gates, and
          reef currents explain electricity.
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

function WelcomePage({ memories: currentMemories }: { memories: StudentMemory[] }) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Compass}>Welcome back</Kicker>
      <h2 className="mt-4 text-4xl font-semibold leading-tight text-stone-950">
        Welcome back, {student.displayName}.
      </h2>
      <p className="mt-4 text-lg leading-8 text-stone-700">
        The Reef remembers that you like visual puzzles, ocean analogies, and
        explanations you can sketch.
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
        <p className="mt-2 text-xl font-semibold">{student.activeQuest}</p>
      </div>
    </div>
  );
}

function StageMapPage({ stages: currentStages }: { stages: Stage[] }) {
  return (
    <div className="flex h-full flex-col">
      <Kicker icon={Map}>Storm Gate map</Kicker>
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
  onPlayNarration,
}: {
  lesson: PrimerLesson;
  isNarrating: boolean;
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
        className="mt-6 inline-flex h-12 items-center justify-center gap-2 rounded-full bg-[#1e6f73] px-5 text-sm font-semibold text-white transition hover:bg-[#195e61]"
      >
        {isNarrating ? (
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

function ProgressPanel({ stages: currentStages }: { stages: Stage[] }) {
  return (
    <section>
      <h3 className="text-sm font-semibold uppercase text-cyan-50/80">
        Stage map
      </h3>
      <div className="mt-3 space-y-2">
        {currentStages.map((stage) => (
          <div
            key={stage.level}
            className="flex items-center justify-between gap-3 rounded-[8px] border border-cyan-100/10 bg-cyan-50/6 px-3 py-3"
          >
            <div>
              <p className="text-sm font-semibold text-stone-100">
                {stage.title.replace("Level ", "L")}
              </p>
              <p className="text-xs text-stone-400">{stage.level}</p>
            </div>
            <StatusBadge status={stage.status} />
          </div>
        ))}
      </div>
    </section>
  );
}

function MemoryPanel({ memories: currentMemories }: { memories: StudentMemory[] }) {
  return (
    <section>
      <h3 className="text-sm font-semibold uppercase text-cyan-50/80">
        What the Primer remembers
      </h3>
      <div className="mt-3 space-y-2">
        {currentMemories.map((memory) => (
          <div
            key={memory.id}
            className="rounded-[8px] border border-cyan-100/10 bg-cyan-50/6 px-3 py-3"
          >
            <p className="text-xs uppercase text-[#d8b86a]">{memory.type}</p>
            <p className="mt-1 text-sm leading-6 text-stone-200">
              {memory.content}
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}

function ContinuityPanel() {
  return (
    <section className="mt-auto rounded-[8px] border border-cyan-100/10 bg-black/18 p-3">
      <h3 className="text-sm font-semibold uppercase text-cyan-50/80">
        Theme bible
      </h3>
      <p className="mt-2 text-sm leading-6 text-stone-300">
        {themeBible.worldSummary}
      </p>
      <p className="mt-3 text-xs uppercase text-stone-500">Guide</p>
      <p className="mt-1 text-sm text-stone-200">
        {themeBible.guide.name}, {themeBible.guide.role}
      </p>
    </section>
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

function StatusBadge({ status }: { status: Stage["status"] }) {
  const className =
    status === "locked"
      ? "border-stone-500/30 text-stone-400"
      : status === "passed"
        ? "border-emerald-300/30 text-emerald-200"
        : "border-[#d8b86a]/40 text-[#f4d98e]";

  return (
    <span
      className={`rounded-full border px-2.5 py-1 text-xs uppercase ${className}`}
    >
      {status}
    </span>
  );
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
    .map((item, index) => {
      const memory = asRecord(item) as BackendMemory | null;
      const content = memory?.content;
      if (!content) {
        return null;
      }

      return {
        id: `${content}-${index}`,
        type: memory.memory_type ?? memory.memoryType ?? "knowledge",
        content,
        tags: Array.isArray(memory.tags) ? memory.tags : [],
      } satisfies StudentMemory;
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

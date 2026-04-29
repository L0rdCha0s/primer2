import { BookOpen, Brain, Compass, Feather, Sparkles } from "lucide-react";

type LessonGenerationOverlayProps = {
  learnerName: string;
  status: string;
  topicHint: string;
};

const preparationSteps = [
  "Reading your profile",
  "Choosing a story path",
  "Sketching the first idea",
];

export function LessonGenerationOverlay({
  learnerName,
  status,
  topicHint,
}: LessonGenerationOverlayProps) {
  const cleanTopic = topicHint.trim() || "your first path";

  return (
    <div className="lesson-generation-overlay" aria-live="polite" role="status">
      <div className="lesson-generation-spread">
        <div className="lesson-generation-scene" aria-hidden="true">
          <div className="lesson-generation-aurora" />
          <div className="lesson-generation-orbit">
            <span className="lesson-generation-node lesson-generation-node-one">
              <Brain className="h-5 w-5" strokeWidth={1.8} />
            </span>
            <span className="lesson-generation-node lesson-generation-node-two">
              <Compass className="h-5 w-5" strokeWidth={1.8} />
            </span>
            <span className="lesson-generation-node lesson-generation-node-three">
              <Sparkles className="h-5 w-5" strokeWidth={1.8} />
            </span>
          </div>
          <div className="lesson-generation-book">
            <div className="lesson-generation-cover" />
            <div className="lesson-generation-page lesson-generation-page-one" />
            <div className="lesson-generation-page lesson-generation-page-two" />
            <div className="lesson-generation-page lesson-generation-page-three" />
            <BookOpen className="lesson-generation-book-icon" strokeWidth={1.6} />
          </div>
          <div className="lesson-generation-ribbon lesson-generation-ribbon-one">
            <Feather className="h-4 w-4" strokeWidth={1.8} />
            <span>story</span>
          </div>
          <div className="lesson-generation-ribbon lesson-generation-ribbon-two">
            <Sparkles className="h-4 w-4" strokeWidth={1.8} />
            <span>diagram</span>
          </div>
        </div>

        <div className="lesson-generation-copy">
          <p className="lesson-generation-kicker">Primer is preparing</p>
          <h2>
            A new lesson is taking shape for{" "}
            <span>{learnerName || "this learner"}</span>.
          </h2>
          <p className="lesson-generation-topic">{cleanTopic}</p>
          <p className="lesson-generation-status">{status}</p>

          <div className="lesson-generation-steps">
            {preparationSteps.map((step, index) => (
              <span key={step} style={{ animationDelay: `${index * 0.42}s` }}>
                {step}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

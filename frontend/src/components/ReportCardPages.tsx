"use client";

import {
  BarChart3,
  ClipboardList,
  ExternalLink,
  RefreshCw,
  Sparkles,
  Target,
  TriangleAlert,
  X,
} from "lucide-react";
import { useEffect, type ComponentType, type ReactNode } from "react";
import {
  type CurriculumCoverage,
  type StudentReportCard,
  coverageStatusLabel,
  reportCardStats,
} from "@/lib/report-card";

type ReportCardPageProps = {
  reportCard: StudentReportCard | null;
  status: string;
};

type ReportCardDialogProps = ReportCardPageProps & {
  isRefreshing: boolean;
  onClose: () => void;
  onRefresh: () => void;
};

export function ReportCardDialog({
  isRefreshing,
  onClose,
  onRefresh,
  reportCard,
  status,
}: ReportCardDialogProps) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="report-card-title"
      className="fixed inset-0 z-[55] flex items-center justify-center bg-[#071111]/94 p-3 text-cyan-50 backdrop-blur-md sm:p-5"
      onClick={onClose}
    >
      <div
        className="flex max-h-[calc(100svh-1.5rem)] w-full max-w-6xl flex-col overflow-hidden rounded-[8px] border border-cyan-100/16 bg-[#101919] shadow-2xl shadow-black/40 sm:max-h-[calc(100svh-2.5rem)]"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="flex flex-wrap items-start justify-between gap-3 border-b border-cyan-100/12 px-4 py-4 sm:px-5">
          <div className="min-w-0">
            <p className="inline-flex items-center gap-2 text-xs uppercase text-[#d8b86a]">
              <ClipboardList className="h-3.5 w-3.5" />
              Report
            </p>
            <h2
              id="report-card-title"
              className="mt-1 text-2xl font-semibold leading-tight text-stone-50"
            >
              {reportCard
                ? `${reportCard.displayName}'s learning report`
                : "Learning report"}
            </h2>
            <p className="mt-1 text-sm leading-6 text-stone-400">{status}</p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <RefreshReportButton
              isRefreshing={isRefreshing}
              onRefresh={onRefresh}
            />
            <button
              type="button"
              aria-label="Close report card"
              onClick={onClose}
              className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-cyan-100/15 text-cyan-50/80 transition hover:text-cyan-50"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </header>

        <div className="grid min-h-0 flex-1 gap-3 overflow-y-auto p-3 sm:p-4 lg:grid-cols-2">
          <section className="min-h-[520px] rounded-[8px] bg-[#f3ead2] p-5 text-stone-950 shadow-inner shadow-white/10 sm:p-6">
            <ReportCardStudentPage reportCard={reportCard} status={status} />
          </section>
          <section className="min-h-[520px] rounded-[8px] bg-[#f3ead2] p-5 text-stone-950 shadow-inner shadow-white/10 sm:p-6">
            <ReportCardParentPage reportCard={reportCard} status={status} />
          </section>
        </div>
      </div>
    </div>
  );
}

export function ReportCardStudentPage({
  reportCard,
  status,
}: ReportCardPageProps) {
  const stats = reportCardStats(reportCard);

  return (
    <div className="flex h-full flex-col">
      <KickerLite icon={ClipboardList}>Report card</KickerLite>

      <h2 className="mt-4 text-3xl font-semibold leading-tight text-stone-950">
        {reportCard ? `${reportCard.displayName}'s learning trail` : "Learning trail"}
      </h2>
      <p className="mt-3 text-base leading-7 text-stone-700">
        {reportCard?.studentSummary ??
          "Complete a lesson and stagegate to add the first report-card evidence."}
      </p>

      <div className="mt-5 grid grid-cols-3 overflow-hidden rounded-[8px] border border-stone-300 bg-white/60">
        <Metric label="Topics" value={stats.topicCount} />
        <Metric label="Stagegates" value={stats.attemptCount} />
        <Metric label="Passed" value={stats.passedAttemptCount} />
      </div>

      <div className="mt-5 space-y-3">
        {(reportCard?.strengths.length ? reportCard.strengths : [status])
          .slice(0, 3)
          .map((strength) => (
            <div
              key={strength}
              className="flex gap-3 border-l-2 border-[#1e6f73] bg-white/50 px-4 py-3"
            >
              <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-[#1e6f73]" />
              <p className="text-sm leading-6 text-stone-800">{strength}</p>
            </div>
          ))}
      </div>

      <div className="mt-auto rounded-[8px] bg-[#173b3b] p-4 text-stone-50">
        <p className="text-xs uppercase text-cyan-100/75">Next step</p>
        <p className="mt-2 text-base font-semibold leading-6">
          {reportCard?.nextSteps[0] ??
            "Complete the first stagegate to create reportable evidence."}
        </p>
      </div>
    </div>
  );
}

export function ReportCardParentPage({
  reportCard,
  status,
}: ReportCardPageProps) {
  const coverage = reportCard?.curriculumCoverage ?? [];

  return (
    <div className="flex h-full flex-col">
      <KickerLite icon={BarChart3}>Curriculum links</KickerLite>

      <h2 className="mt-4 text-3xl font-semibold leading-tight text-stone-950">
        {reportCard?.yearLevel.label ?? "Year level pending"}
      </h2>
      <p className="mt-3 text-sm leading-6 text-stone-700">
        {reportCard?.parentSummary ?? status}
      </p>
      {reportCard?.yearLevel.note ? (
        <div className="mt-3 flex gap-2 rounded-[8px] border border-amber-300/60 bg-[#fff8df] px-3 py-2 text-xs leading-5 text-[#62480a]">
          <TriangleAlert className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          {reportCard.yearLevel.note}
        </div>
      ) : null}

      <div className="mt-5 space-y-3">
        {coverage.length > 0 ? (
          coverage.map((item) => (
            <CurriculumCoverageRow key={item.referenceId} item={item} />
          ))
        ) : (
          <div className="rounded-[8px] border border-stone-300 bg-white/60 p-4">
            <p className="text-sm leading-6 text-stone-700">{status}</p>
          </div>
        )}
      </div>

      <div className="mt-auto grid gap-2 border-t border-stone-300 pt-3">
        {(reportCard?.growthAreas.length
          ? reportCard.growthAreas
          : ["No growth areas are available until a stagegate is submitted."]
        )
          .slice(0, 2)
          .map((growthArea) => (
            <div key={growthArea} className="flex gap-2 text-xs leading-5 text-stone-600">
              <Target className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[#1e6f73]" />
              {growthArea}
            </div>
          ))}
      </div>
    </div>
  );
}

function RefreshReportButton({
  isRefreshing,
  onRefresh,
}: {
  isRefreshing: boolean;
  onRefresh: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onRefresh}
      disabled={isRefreshing}
      className="inline-flex h-9 items-center gap-2 rounded-full bg-[#d8b86a] px-3 text-xs font-semibold text-[#1c211d] transition hover:bg-[#e5c879] disabled:cursor-wait disabled:opacity-70"
    >
      <RefreshCw className={`h-3.5 w-3.5 ${isRefreshing ? "animate-spin" : ""}`} />
      Refresh
    </button>
  );
}

function CurriculumCoverageRow({ item }: { item: CurriculumCoverage }) {
  const isCovered = item.status === "covered";
  const isDeveloping = item.status === "developing";

  return (
    <div className="rounded-[8px] border border-stone-300 bg-white/62 p-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-stone-950">
            {item.learningArea}
          </p>
          <p className="mt-1 text-xs leading-5 text-stone-600">{item.strand}</p>
        </div>
        <span
          className={`shrink-0 rounded-full px-2.5 py-1 text-[11px] font-semibold uppercase ${
            isCovered
              ? "bg-[#1e6f73] text-white"
              : isDeveloping
                ? "bg-[#fff8df] text-[#654f12]"
                : "bg-stone-100 text-stone-500"
          }`}
        >
          {coverageStatusLabel(item.status)}
        </span>
      </div>
      <p className="mt-2 line-clamp-2 text-xs leading-5 text-stone-700">
        {item.parentNote}
      </p>
      <div className="mt-3 flex items-center justify-between gap-3 border-t border-stone-200 pt-2 text-[11px] uppercase text-stone-500">
        <span>
          {item.evidenceCount} evidence topic{item.evidenceCount === 1 ? "" : "s"}
        </span>
        {item.sourceUrl ? (
          <a
            href={item.sourceUrl}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 font-semibold text-[#1e6f73] hover:text-[#195e61]"
          >
            AC v9
            <ExternalLink className="h-3 w-3" />
          </a>
        ) : null}
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="border-r border-stone-300 px-3 py-3 text-center last:border-r-0">
      <p className="text-2xl font-semibold text-stone-950">{value}</p>
      <p className="mt-1 text-[11px] uppercase text-stone-500">{label}</p>
    </div>
  );
}

function KickerLite({
  icon: Icon,
  children,
}: {
  icon: ComponentType<{ className?: string }>;
  children: ReactNode;
}) {
  return (
    <div className="inline-flex items-center gap-2 text-xs uppercase text-[#1e6f73]">
      <Icon className="h-3.5 w-3.5" />
      {children}
    </div>
  );
}

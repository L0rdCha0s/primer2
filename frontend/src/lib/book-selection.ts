export const pageTurnEdgeWidthPx = 54;

type HorizontalBounds = {
  left: number;
  right: number;
};

export function isPointerNearPageTurnEdge(
  clientX: number,
  bounds: HorizontalBounds,
  edgeWidth = pageTurnEdgeWidthPx,
): boolean {
  const width = Math.max(0, bounds.right - bounds.left);
  if (width <= 0) {
    return false;
  }

  const safeEdgeWidth = Math.min(Math.max(24, edgeWidth), width / 4);
  return (
    clientX >= bounds.left &&
    clientX <= bounds.right &&
    (clientX - bounds.left <= safeEdgeWidth ||
      bounds.right - clientX <= safeEdgeWidth)
  );
}

export function normalizeSelectedBookText(text: string, maxLength = 720): string {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }

  return `${normalized.slice(0, Math.max(0, maxLength - 1)).trimEnd()}...`;
}

export function selectedTextInfographicPrompt(
  selectedText: string,
  lessonTopic: string,
): string {
  const cleanText = normalizeSelectedBookText(selectedText, 520);
  const cleanTopic = normalizeSelectedBookText(lessonTopic, 120);

  return [
    `Create a small, age-appropriate educational infographic about this selected passage from the lesson on ${cleanTopic || "the current topic"}.`,
    `Selected passage: ${cleanText}`,
    "Use clear labels, one simple visual metaphor, and no tiny text.",
  ].join(" ");
}

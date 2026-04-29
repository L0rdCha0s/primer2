export type StageLevel = "intuition" | "mechanism" | "transfer";

export type Stage = {
  level: StageLevel;
  title: string;
  status: "available" | "locked" | "passed";
  description: string;
};

export type StudentMemory = {
  id: string;
  type: "preference" | "knowledge" | "misconception" | "interest" | "history";
  content: string;
  tags: string[];
  assertionId?: string;
  subject?: string;
  predicate?: string;
  validFrom?: string;
  validTo?: string;
  knownFrom?: string;
  knownTo?: string;
  source?: string;
};

export type InfographicSpec = {
  title: string;
  subtitle: string;
  theme: {
    name: string;
    visualMotif: string;
  };
  panels: Array<{
    heading: string;
    body: string;
    icon:
      | "cloud"
      | "bolt"
      | "water"
      | "gear"
      | "atom"
      | "leaf"
      | "brain"
      | "magnifier"
      | "map"
      | "spark";
    visualMetaphor: string;
  }>;
  callouts: Array<{
    label: string;
    text: string;
  }>;
  keyTerms: Array<{
    term: string;
    definition: string;
  }>;
  narrationScript: string;
};

export type StagegateResult = {
  passed: boolean;
  score: number;
  rubric: {
    accuracy: number;
    causalReasoning: number;
    vocabulary: number;
    transfer: number;
  };
  masteryEvidence: string[];
  gaps: string[];
  feedbackToStudent: string;
  nextLevelUnlocked?: "mechanism" | "transfer" | "complete";
};

export const student = {
  displayName: "Mina",
  ageBand: "11-13",
  currentTheme: "The Clockwork Reef",
  activeQuest: "Open the Storm Gate by understanding lightning.",
  interests: ["marine biology", "drawing", "puzzles"],
  preferredExplanationStyle: "visual",
};

export const themeBible = {
  worldSummary:
    "An underwater city where concepts become machines, currents, creatures, and quest gates.",
  visualStyle: "inked reef diagrams, brass mechanisms, bioluminescent charge paths",
  guide: {
    name: "Tala",
    role: "reef guide",
    voice: "calm, curious, precise",
  },
};

export const stages: Stage[] = [
  {
    level: "intuition",
    title: "Level 1: Intuition",
    status: "available",
    description: "Explain lightning in plain language.",
  },
  {
    level: "mechanism",
    title: "Level 2: Mechanism",
    status: "locked",
    description: "Order the causal process and name the parts.",
  },
  {
    level: "transfer",
    title: "Level 3: Transfer",
    status: "locked",
    description: "Apply the idea to static shock and new cases.",
  },
];

export const memories: StudentMemory[] = [
  {
    id: "visual-puzzles",
    type: "preference",
    content: "Learner likes visual puzzles and diagram-first explanations.",
    tags: ["style", "visual"],
  },
  {
    id: "water-current",
    type: "preference",
    content: "Ocean-current analogies help the learner compare invisible forces.",
    tags: ["analogy", "electricity"],
  },
  {
    id: "energy-force",
    type: "misconception",
    content: "Learner previously struggled to separate energy from force.",
    tags: ["physics", "misconception"],
  },
  {
    id: "energy-intuition",
    type: "knowledge",
    content: "Learner passed Energy: Intuition yesterday.",
    tags: ["mastery", "energy"],
  },
];

export const tutorScene = {
  prompt: "Why does lightning happen?",
  storyScene:
    "In the Clockwork Reef, storm-cloud whales carry tiny charged pearls. As they swim and collide, the pearls sort themselves apart: bright pearls gather high in the cloud, shadow pearls gather low. When the pearl pressure grows strong enough, the air opens a glowing path.",
  plainExplanation:
    "Lightning happens when separated electric charges build a strong electric field. Once the field is strong enough to push through air, charge suddenly moves and releases energy as a bright flash.",
  analogy:
    "Think of voltage like pressure in reef pipes. Current is the water moving through the pipe. Lightning is what happens when the pressure becomes so great that the water bursts through a new path.",
  check:
    "Can you explain lightning using these three ideas: charges separate, the electric field grows, and charge suddenly moves?",
};

export const lightningInfographic: InfographicSpec = {
  title: "How Lightning Forms",
  subtitle: "A four-step journey from charge to flash",
  theme: {
    name: "The Clockwork Reef",
    visualMotif: "cloud-whales, charged pearls, brass reef gates, glowing currents",
  },
  panels: [
    {
      heading: "Clouds separate charge",
      body: "Ice particles collide inside storm clouds and sort positive and negative charge into different regions.",
      icon: "cloud",
      visualMetaphor: "cloud-whales sorting bright and shadow pearls",
    },
    {
      heading: "Electric field builds",
      body: "The separated charges pull on each other across the air, building a stronger electric field.",
      icon: "spark",
      visualMetaphor: "pearl pressure straining the reef pipes",
    },
    {
      heading: "Air breaks down",
      body: "When the field becomes strong enough, air stops insulating and a path opens for charge.",
      icon: "gear",
      visualMetaphor: "a brass lock clicking open in the storm gate",
    },
    {
      heading: "Charge rushes",
      body: "Charge moves suddenly through the new path, releasing energy as light, heat, and thunder.",
      icon: "bolt",
      visualMetaphor: "a glowing current racing through the reef",
    },
  ],
  callouts: [
    {
      label: "Key idea",
      text: "Lightning is a sudden movement of electric charge through air.",
    },
    {
      label: "Watch for",
      text: "Voltage is pressure; current is moving charge; energy is what gets released.",
    },
  ],
  keyTerms: [
    {
      term: "Voltage",
      definition: "Electric pressure that pushes charge.",
    },
    {
      term: "Current",
      definition: "The flow of electric charge.",
    },
    {
      term: "Electric field",
      definition: "The invisible push or pull around charge.",
    },
  ],
  narrationScript:
    "In the Clockwork Reef, storm-cloud whales sort charged pearls into bright and shadow layers. The farther apart the pearls gather, the more electric pressure builds between them. At first, air acts like a sealed reef gate. But when the pressure is strong enough, that gate opens. Charge rushes through the new path, and the stored energy bursts out as light, heat, and thunder. That sudden rush of charge is lightning.",
};

export const seededStagegateResult: StagegateResult = {
  passed: true,
  score: 0.84,
  rubric: {
    accuracy: 0.86,
    causalReasoning: 0.82,
    vocabulary: 0.78,
    transfer: 0.88,
  },
  masteryEvidence: [
    "Explained that charges separate before lightning.",
    "Connected the flash to sudden charge movement.",
    "Used voltage as pressure without calling it energy.",
  ],
  gaps: ["Keep practicing the difference between voltage and current."],
  feedbackToStudent:
    "You opened the Storm Gate. Your answer showed the cause-and-effect chain from separated charge to sudden movement.",
  nextLevelUnlocked: "mechanism",
};

export const unlockedMemory: StudentMemory = {
  id: "lightning-intuition",
  type: "knowledge",
  content:
    "Mina understands lightning as sudden charge movement at the intuition level.",
  tags: ["mastery", "electricity", "lightning"],
};

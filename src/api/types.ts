/** Wire types mirroring `src-tauri/src/db/models.rs` (camelCase via serde). */

export type ItemType = "task" | "reminder" | "note" | "idea" | "project_ref" | "event";
export type Status = "inbox" | "next" | "doing" | "done";
export type Priority = "low" | "normal" | "high" | "urgent";
export type BoardKind = "project" | "personal";

export const ITEM_TYPES: ItemType[] = ["task", "reminder", "note", "idea", "event", "project_ref"];
export const STATUSES: Status[] = ["inbox", "next", "doing", "done"];
export const PRIORITIES: Priority[] = ["low", "normal", "high", "urgent"];

/** RFC 3339 timestamp in UTC. */
export type Iso = string;

/** Something AI capture was unsure about, shown as a chip until resolved. */
export interface CaptureHint {
  datePhrase?: string;
  lowConfidence?: boolean;
  suggestedProject?: string;
  /** An open card this capture closely matches ("Looks like …"). */
  similarTo?: string;
  similarTitle?: string;
}

export interface Item {
  id: string;
  kind: ItemType;
  title: string;
  body: string | null;
  status: Status;
  priority: Priority;
  dueAt: Iso | null;
  endsAt: Iso | null;
  remindAt: Iso | null;
  snoozedUntil: Iso | null;
  recurrence: string | null;
  projectId: string | null;
  createdAt: Iso;
  updatedAt: Iso;
  completedAt: Iso | null;
  tags: string[];
  linkCount: number;
  hint: CaptureHint | null;
  /** An AI capture is still turning this placeholder into its final card. */
  capturing: boolean;
  /** Cards: pinned cards sort first. */
  pinned?: boolean;
  /** Cards: set while the card is in the Archive. */
  archivedAt?: Iso | null;
  /** Cards: set while the card is in the Trash. */
  deletedAt?: Iso | null;
  subtasksDone?: number;
  subtasksTotal?: number;
  attachmentCount?: number;
}

export interface NewItem {
  kind: ItemType;
  title: string;
  body?: string | null;
  priority?: Priority | null;
  status?: Status | null;
  dueAt?: Iso | null;
  endsAt?: Iso | null;
  remindAt?: Iso | null;
  recurrence?: string | null;
  projectId?: string | null;
  boardId?: string | null;
  tags?: string[] | null;
  x?: number | null;
  y?: number | null;
}

/** Omit a key to leave it alone; send `null` to clear a nullable field. */
export interface ItemPatch {
  kind?: ItemType;
  title?: string;
  body?: string | null;
  status?: Status;
  priority?: Priority;
  dueAt?: Iso | null;
  endsAt?: Iso | null;
  remindAt?: Iso | null;
  snoozedUntil?: Iso | null;
  recurrence?: string | null;
  projectId?: string | null;
  tags?: string[];
  hint?: CaptureHint | null;
}

export type ItemFilter =
  | { kind: "inbox" }
  | { kind: "today" }
  | { kind: "upcoming"; days?: number }
  | { kind: "ideas" }
  | { kind: "notes" }
  | { kind: "project"; id: string }
  | { kind: "board"; id: string }
  | { kind: "all" };

export interface Counts {
  inbox: number;
  today: number;
  upcoming: number;
  ideas: number;
  notes: number;
  byProject: Record<string, number>;
}

export const EMPTY_COUNTS: Counts = {
  inbox: 0,
  today: 0,
  upcoming: 0,
  ideas: 0,
  notes: 0,
  byProject: {},
};

export interface Project {
  id: string;
  name: string;
  color: string;
  boardId: string;
  createdAt: Iso;
}

export interface Progress {
  done: number;
  total: number;
}

export interface Board {
  id: string;
  name: string;
  kind: BoardKind;
  createdAt: Iso;
}

export interface Placement {
  itemId: string;
  boardId: string;
  x: number;
  y: number;
}

export interface Link {
  id: string;
  fromItem: string;
  toItem: string;
  label: string | null;
  createdAt: Iso;
}

export interface ItemsChanged {
  ids: string[];
}

// ── AI ────────────────────────────────────────────────────────────────────

export type AiProvider = "builtin" | "ollama";
export type AiPhase = "idle" | "loading" | "processing" | "error";

export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  file: string;
  sizeBytes: number;
  ramGb: number;
  recommended: boolean;
  installed: boolean;
}

export interface SpeechInfo {
  id: string;
  name: string;
  description: string;
  file: string;
  sizeBytes: number;
  ramGb: number;
  recommended: boolean;
  installed: boolean;
}

export interface DownloadProgress {
  modelId: string;
  stage: "server" | "model" | "speech";
  received: number;
  total: number;
}

export interface AiState {
  provider: AiProvider;
  modelId: string;
  ollamaModel: string | null;
  ollamaUrl: string;
  ready: boolean;
  phase: AiPhase;
  message: string | null;
  serverInstalled: boolean;
  serverRunning: boolean;
  models: ModelInfo[];
  download: DownloadProgress | null;
  ollamaAvailable: boolean;
  ollamaModels: string[];
  pending: number;
  speechModelId: string;
  speechModels: SpeechInfo[];
  speechReady: boolean;
}

export interface AiConfigPatch {
  provider?: AiProvider;
  modelId?: string;
  ollamaModel?: string;
  speechModelId?: string;
}

export interface CaptureContext {
  projectId?: string | null;
  boardId?: string | null;
  x?: number | null;
  y?: number | null;
  status?: Status | null;
}

export interface CaptureResult {
  captureId: string;
  item: Item;
}

export interface CaptureEvent {
  captureId: string;
  status: "done" | "failed";
  itemIds: string[];
  error: string | null;
  latencyMs: number | null;
}

// ── Desktop presence ─────────────────────────────────────────────────────

export type VoicePhase =
  "idle" | "listening" | "transcribing" | "editing" | "saved" | "empty" | "unavailable" | "error";

export interface VoiceState {
  phase: VoicePhase;
  text: string | null;
  message: string | null;
  captureId: string | null;
  itemId: string | null;
}

export interface ReminderToast {
  /** Item id, or "away" for the grouped "while you were away" toast. */
  id: string;
  items: Item[];
  missed: boolean;
  /** Present on a "filed in…" toast shown after a capture. */
  filed?: Filed | null;
  /** The once-a-day "today" summary. */
  brief?: boolean;
}

/** Where a card lives: a project, a personal board or the Inbox. */
export type Place =
  { kind: "project"; id: string } | { kind: "board"; id: string } | { kind: "inbox" };

export interface Filed {
  captureId: string;
  place: Place;
  placeName: string;
  /** True once the user moved it from the toast. */
  moved: boolean;
}

export interface BackupInfo {
  dir: string;
  count: number;
  lastAt: string | null;
}

export type PanelName = "orb" | "overlay" | "capture" | "panel" | "toasts";

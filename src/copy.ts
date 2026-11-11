/** All user-facing strings. English only in v1; kept in one module so i18n can be added later. */
export const copy = {
  appName: "NOMA",
  tagline: "YOUR SECOND MIND.",
  nav: {
    inbox: "Inbox",
    today: "Today",
    upcoming: "Upcoming",
    ideas: "Ideas",
    notes: "Notes",
    projects: "Projects",
    boards: "Personal boards",
    settings: "Settings",
    collapse: "Collapse sidebar",
    expand: "Expand sidebar",
  },
  viewModes: {
    canvas: "Canvas",
    kanban: "Kanban",
    list: "List",
    calendar: "Calendar",
    timeline: "Timeline",
  },
  soon: "Soon",
  capture: "Capture",
  search: "Search",
  searchHint: "⌘K",
  aiStatus: "AI status",
  aiIdle: "Idle",
  aiListening: "Listening",
  aiProcessing: "Thinking",
  aiSuccess: "Captured",
  aiError: "Something went wrong",
  orbHere: "NOMA is here",
  orbHint: (voice: string) => `${voice} to talk`,
  profile: "Profile and settings",
  cards: (n: number) => `${n} ${n === 1 ? "card" : "cards"}`,
  empty: {
    inbox: {
      title: "Your mind is clear. For now.",
      body: "Inbox is empty. Anything you tell NOMA lands here first.",
    },
    today: {
      title: "Nothing on today's plate.",
      body: "No tasks, reminders or events due today.",
    },
    upcoming: {
      title: "Nothing demanding your attention.",
      body: "No reminders in the next 7 days.",
    },
    ideas: {
      title: "The next idea hasn't arrived yet.",
      body: "Say it out loud when it does. NOMA keeps it.",
    },
    notes: {
      title: "A blank page, on purpose.",
      body: "Notes you capture live here, searchable and connected.",
    },
    project: {
      title: "Every great thing starts somewhere.",
      body: "Empty project — press N to add the first card.",
    },
    board: {
      title: "Room to think.",
      body: "This board is empty. Drop a card anywhere.",
    },
    settings: {
      title: "Settings",
      body: "",
    },
  },
} as const;

export type EmptyKind = keyof typeof copy.empty;

export const kindLabel = {
  task: "Task",
  reminder: "Reminder",
  note: "Note",
  idea: "Idea",
  project_ref: "Project",
  event: "Event",
} as const;

export const statusLabel = {
  inbox: "Inbox",
  next: "Next",
  doing: "Doing",
  done: "Done",
} as const;

export const priorityLabel = {
  low: "Low",
  normal: "Normal",
  high: "High",
  urgent: "Urgent",
} as const;

export const cardCopy = {
  open: "Open",
  duplicate: "Duplicate",
  delete: "Delete",
  complete: "Mark complete",
  reopen: "Mark not done",
  linked: (n: number) => `${n} linked`,
  noTitle: "Untitled",
  repeats: "Repeats",
} as const;

export const listCopy = {
  overdue: "Overdue",
  morning: "Morning",
  afternoon: "Afternoon",
  evening: "Evening",
  today: "Today",
  tomorrow: "Tomorrow",
  thisWeek: "This week",
  later: "Later",
  noDate: "No date",
  checked: "Checked",
} as const;

export const quickAddCopy = {
  placeholder: "What's on your mind?",
  autoPlaceholder: "Say it like you'd say it — “call Marta tomorrow at 10”",
  auto: "Auto",
  autoReady: "NOMA sorts it out · ↵",
  autoPending: "Saved now, sorted when AI is set up · ↵",
  hint: "↵ to add · Esc to close",
  add: "Add",
} as const;

export const hintCopy = {
  checkDate: (phrase: string) => `Check date · ${phrase}`,
  checkThis: "Check this",
  newProject: (name: string) => `New project? ${name}`,
  dismiss: "Dismiss",
  reading: "Reading…",
} as const;

export const settingsCopy = {
  title: "Settings",
  aiTitle: "On-device AI",
  aiLead:
    "NOMA reads your captures with a small language model that runs entirely on this Mac. Nothing you say leaves your computer.",
  provider: "Engine",
  builtin: "Built-in",
  ollama: "Ollama",
  ollamaLead: "Developer option: use a model already pulled in a local Ollama.",
  ollamaMissing: "Ollama isn't running on this Mac.",
  ollamaPick: "Choose a model",
  download: "Download",
  cancel: "Cancel",
  remove: "Remove",
  use: "Use",
  inUse: "In use",
  recommended: "Recommended",
  ram: (gb: number) => `~${gb} GB memory`,
  size: (bytes: number) => `${(bytes / 1e9).toFixed(1)} GB`,
  downloading: (stage: string, pct: number) =>
    stage === "server" ? `Preparing engine… ${pct}%` : `Downloading… ${pct}%`,
  pending: (n: number) => `${n} capture${n === 1 ? "" : "s"} waiting for the model.`,
  ready: "Ready. Captures are sorted as you make them.",
  notReady: "Download a model to let NOMA sort your captures.",
  voiceTitle: "Voice",
  voiceLead: (voice: string) =>
    `Hold ${voice} anywhere and talk; release to save. Tap it once to start and tap again to finish. Speech is transcribed on this Mac by a Whisper model.`,
  voiceReady: (voice: string) => `Ready. Hold ${voice} anywhere.`,
  voiceNotReady: "Download a speech model to talk to NOMA.",
  display: "While I talk",
  displayHint:
    "Orb only keeps the screen calm: the orb pulses, and everything else happens in the background.",
  displays: { orb: "Orb only", overlay: "Full panel" },
  language: "Spoken language",
  languageHint: "Pinning the language makes short phrases much more accurate than auto-detect.",
  languages: { it: "Italiano", en: "English", auto: "Auto-detect" },
  autoStop: "Stop when I stop talking",
  autoStopHint:
    "Off by default: you decide when you're done, by releasing the key or tapping it again.",
  desktopTitle: "On your desktop",
  openAtLogin: "Open at login",
  openAtLoginHint:
    "NOMA starts quietly in the menu bar, so the voice shortcut works right after you log in. ⌘Q only closes the window; use Quit NOMA in the menu bar to exit.",
  orbToggle: "Floating orb",
  orbHint: "A small orb that stays on top of other apps when NOMA is in the background.",
  remindersPaused: "Pause reminders",
  remindersPausedHint: "Reminders still fire later — nothing is lost.",
  nativeNotifications: "System notifications",
  nativeHint: "Also notify through macOS when NOMA isn't in front.",
  filedToast: "Show where captures go",
  filedToastHint:
    "After a voice or quick capture, a small note says where NOMA filed it, with Undo and Move. Moving it teaches NOMA for next time.",
  brief: "Morning brief",
  briefHint:
    "Once a day, the first time you're at your Mac after 7:00, NOMA shows what's on today.",
  dataTitle: "Your data",
  dataLead:
    "Everything lives on this Mac. NOMA keeps a daily backup (the last 10) and can export all your cards as Markdown and JSON.",
  lastBackup: (when: string | null) => (when ? `Last backup: ${when}` : "No backup yet"),
  backupNow: "Back up now",
  showBackups: "Show backups",
  exportData: "Export…",
  exported: "Exported to Downloads",
  shortcuts: "Shortcuts",
  shortcutHint: "Click Change, press the keys you want — you'll see them live — then Save.",
  shortcutChange: "Change",
  shortcutSave: "Save",
  shortcutReset: "Reset",
  shortcutPress: "Press the keys…",
  shortcutNeedsModifier: "Hold ⌥, ⌘, ⌃ or ⇧ together with a key.",
  shortcutVoice: "Talk to NOMA",
  shortcutCapture: "Quick capture",
  appearance: "Appearance",
  theme: { dark: "Dark", light: "Light", system: "Match system" },
  about: "About",
  version: "NOMA — your second mind.",
} as const;

export const detailCopy = {
  close: "Close",
  title: "Title",
  notes: "Notes",
  notesPlaceholder: "Add details…",
  type: "Type",
  status: "Status",
  priority: "Priority",
  due: "Due",
  starts: "Starts",
  ends: "Ends",
  reminder: "Reminder",
  repeat: "Repeat",
  repeatNeedsDate: "Set a date to repeat",
  project: "Project",
  noProject: "No project",
  tags: "Tags",
  tagsPlaceholder: "Add a tag…",
  created: "Created",
  clear: "Clear",
} as const;

export const paletteCopy = {
  placeholder: "Search cards or type a command…",
  goTo: "Go to",
  actions: "Actions",
  cards: "Cards",
  noResults: "Nothing matches. Press ↵ to add it as a task.",
} as const;

export const sidebarCopy = {
  newProject: "New project",
  newBoard: "New board",
  projectPlaceholder: "Project name",
  boardPlaceholder: "Board name",
} as const;

export const desktopCopy = {
  listening: "Listening…",
  transcribing: "Transcribing…",
  editing: "Edit before saving",
  saved: "Captured",
  empty: "Didn't catch that. Try again?",
  unavailable: "Voice needs a speech model. Download one in Settings → Voice.",
  error: "Voice capture failed",
  save: "Save",
  edit: "Edit",
  cancel: "Cancel",
  holdAnywhere: "Hold",
  thinking: "Sorting it out…",
  capturePlaceholder: "Capture a thought…",
  auto: "Auto",
  inbox: "Inbox",
  captureSaved: "Saved to NOMA",
  today: "Today",
  left: (n: number) => `${n} left`,
  allClear: "Nothing left today.",
  openNoma: "Open NOMA",
  talk: "Talk to NOMA",
  snooze1h: "Snooze 1h",
  tomorrow: "Tomorrow",
  done: "Done",
  dismiss: "Dismiss",
  inMinutes: (m: number) => (m <= 0 ? "Now" : `In ${m} ${m === 1 ? "minute" : "minutes"}`),
  briefTitle: (n: number) => `Today · ${n} ${n === 1 ? "thing" : "things"}`,
  seeToday: "See today",
  filedIn: (place: string) => `Filed in ${place}`,
  movedTo: (place: string) => `Moved to ${place} — NOMA will remember`,
  andMore: (n: number) => `+${n} more`,
  undo: "Undo",
  move: "Move",
  open: "Open",
  moveTo: "Move to",
  missed: (n: number) => `While you were away · ${n} ${n === 1 ? "reminder" : "reminders"}`,
};

export const onboardingCopy = {
  welcomeTitle: "Meet NOMA.",
  welcomeBody:
    "Your second mind. Say or type anything — tasks, reminders, ideas — and NOMA files it for you. Everything stays on this Mac.",
  start: "Get started",
  modelsTitle: "Bring NOMA's mind home",
  modelsBody:
    "NOMA runs two small models locally: one to understand what you say, one to hear you. Nothing leaves your computer.",
  brain: "Understanding",
  ears: "Voice",
  download: "Download",
  downloading: "Downloading…",
  ready: "Ready",
  later: "Do this later",
  next: "Continue",
  tryTitle: "Try it.",
  tryBody: (voice: string, capture: string) =>
    `Hold ${voice} anywhere and say what's on your mind. Release to save. ${capture} to type instead.`,
  loginNote:
    "NOMA now starts quietly at login, so the shortcut always works. Turn it off in Settings → On your desktop.",
  tryNow: (voice: string) => `Try ${voice}`,
  finish: "Start using NOMA",
  step: (i: number, n: number) => `${i} of ${n}`,
};

export const semanticCopy = {
  // Command palette
  askCommand: "Ask NOMA",
  askAbout: (q: string) => `Ask NOMA: “${q}”`,
  related: "Related",
  // Header
  ask: "Ask",
  askHint: "Ask NOMA about your cards",
  // Ask NOMA panel
  title: "Ask NOMA",
  placeholder: "Ask about your cards…",
  intro:
    "Ask anything about your tasks, notes and ideas. NOMA answers from your own cards, right on this Mac.",
  examples: ["What's due this week?", "What did I note about the launch?"],
  thinking: "Thinking…",
  noCards: "I couldn't find anything about that in your cards.",
  failed: (msg: string) => `Couldn't answer: ${msg}`,
  sources: "From your cards",
  openCard: (title: string) => `Open “${title}”`,
  newConversation: "New conversation",
  send: "Ask",
  close: "Close",
  // Settings
  settingsTitle: "Search by meaning",
  settingsLead:
    "A small extra model lets the search palette and Ask NOMA find cards by meaning, not only by exact words. It runs on this Mac.",
  toggle: "Search by meaning",
  toggleHint: "Keeps an index of your cards up to date in the background.",
  notInstalled: "Download the search model to turn it on.",
  off: "Off. Search uses exact words only.",
  indexed: (n: number, total: number) => `Ready · ${n} of ${total} cards indexed`,
  indexing: (n: number, total: number) => `Indexing your cards… ${n} of ${total}`,
  downloading: (pct: number) => `Downloading… ${pct}%`,
  rebuild: "Rebuild index",
};

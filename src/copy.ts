/** All user-facing strings, English (the source locale). Italian lives in copy.it.ts; the exports below follow the active interface language. */
import { it } from "./copy.it";
import { getLocale, type Locale } from "./i18n/locale";

const copy_en = {
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

export type EmptyKind = keyof typeof copy_en.empty;

const kindLabel_en = {
  task: "Task",
  reminder: "Reminder",
  note: "Note",
  idea: "Idea",
  project_ref: "Project",
  event: "Event",
} as const;

const statusLabel_en = {
  inbox: "Inbox",
  next: "Next",
  doing: "Doing",
  done: "Done",
} as const;

const priorityLabel_en = {
  low: "Low",
  normal: "Normal",
  high: "High",
  urgent: "Urgent",
} as const;

const cardCopy_en = {
  open: "Open",
  duplicate: "Duplicate",
  delete: "Delete",
  complete: "Mark complete",
  reopen: "Mark not done",
  linked: (n: number) => `${n} linked`,
  noTitle: "Untitled",
  repeats: "Repeats",
} as const;

const listCopy_en = {
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

const quickAddCopy_en = {
  placeholder: "What's on your mind?",
  autoPlaceholder: "Say it like you'd say it — “call Marta tomorrow at 10”",
  auto: "Auto",
  autoReady: "NOMA sorts it out · ↵",
  autoPending: "Saved now, sorted when AI is set up · ↵",
  hint: "↵ to add · Esc to close",
  add: "Add",
} as const;

const hintCopy_en = {
  checkDate: (phrase: string) => `Check date · ${phrase}`,
  checkThis: "Check this",
  newProject: (name: string) => `New project? ${name}`,
  dismiss: "Dismiss",
  reading: "Reading…",
} as const;

const settingsCopy_en = {
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
  themeLabel: "Theme",
  interfaceLanguage: "Interface language",
  interfaceLanguageHint:
    "The language of menus, buttons and notifications. What you say or type is never translated.",
  interfaceLanguages: { system: "Match system", en: "English", it: "Italiano" },
  about: "About",
  version: "NOMA — your second mind.",
} as const;

const detailCopy_en = {
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

const paletteCopy_en = {
  placeholder: "Search cards or type a command…",
  goTo: "Go to",
  actions: "Actions",
  cards: "Cards",
  noResults: "Nothing matches. Press ↵ to add it as a task.",
} as const;

const sidebarCopy_en = {
  newProject: "New project",
  newBoard: "New board",
  projectPlaceholder: "Project name",
  boardPlaceholder: "Board name",
} as const;

const desktopCopy_en = {
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

const onboardingCopy_en = {
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

const organizeCopy_en = {
  aiFailed: "NOMA couldn't do that right now. Check that the AI model is set up in Settings.",
  resort: {
    button: "Re-sort Inbox",
    working: "Re-sorting…",
    title: "File Inbox cards again with your current projects and boards",
    moved: (n: number) =>
      n === 0 ? "Everything stays in the Inbox" : `Moved ${n} ${n === 1 ? "card" : "cards"}`,
  },
  duplicate: {
    chip: (title: string) => `Looks like ${title}`,
    lead: "This looks like a card you already have:",
    merge: "Merge",
    keepBoth: "Keep both",
    merged: "Merged into the existing card",
    failed: "Couldn't merge these cards",
  },
  breakDown: {
    button: "Break down",
    working: "Breaking down…",
    title: "Ask NOMA for the next steps and add them as linked tasks",
    done: (n: number) => `Added ${n} ${n === 1 ? "step" : "steps"}`,
  },
  brief: {
    next: "Next",
    suggest: "Suggest next action",
    refreshNext: "Suggest again",
    nothingOpen: "Nothing open in this project",
    summarize: "Summarize",
    resummarize: "Regenerate summary",
    working: "Thinking…",
    open: "Open card",
  },
  review: {
    nav: "Review",
    lead: "The last 7 days at a glance.",
    summary: "This week",
    summarize: "Write my summary",
    resummarize: "Write it again",
    working: "Writing…",
    completed: "Completed",
    overdue: "Overdue",
    forgotten: "Forgotten",
    forgottenHint: "Untouched for 30+ days",
    emptyCompleted: "Nothing finished this week yet.",
    emptyOverdue: "Nothing overdue. Nice.",
    emptyForgotten: "Nothing gathering dust.",
    loading: "Loading…",
  },
} as const;

const semanticCopy_en = {
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

const cardsCopy_en = {
  pin: "Pin",
  unpin: "Unpin",
  pinned: "Pinned",
  archive: "Archive",
  unarchive: "Restore from Archive",
  archived: (title: string) => `Archived “${title}”`,
  trashed: (title: string) => `Moved “${title}” to Trash`,
  restored: (title: string) => `Restored “${title}”`,
  undo: "Undo",
  restore: "Restore",
  deleteForever: "Delete forever",
  inArchive: "This card is in the Archive.",
  inTrash: "This card is in the Trash.",
  checklist: "Checklist",
  subtasksProgress: (done: number, total: number) => `${done}/${total}`,
  addSubtask: "Add an item…",
  removeSubtask: "Remove item",
  moveUp: "Move up",
  moveDown: "Move down",
  attachments: "Attachments",
  attach: "Attach files",
  attachmentCount: (n: number) => `${n} ${n === 1 ? "attachment" : "attachments"}`,
  openFile: "Open",
  showInFinder: "Show in Finder",
  removeFile: "Remove",
  fileSize: (bytes: number) =>
    bytes < 1024
      ? `${bytes} B`
      : bytes < 1024 * 1024
        ? `${Math.round(bytes / 1024)} KB`
        : `${(bytes / 1024 / 1024).toFixed(1)} MB`,
  editNotes: "Edit notes",
  preview: "Preview",
  markdownHint: "Markdown: **bold**, _italic_, `code`, - lists, [links](https://…)",
  nav: { archive: "Archive", trash: "Trash" },
  archiveEmpty: {
    title: "Nothing archived.",
    body: "Archive cards, projects and boards you're done with; they wait here.",
  },
  trashEmpty: {
    title: "Trash is empty.",
    body: "Deleted cards stay here for 30 days, then they're gone for good.",
  },
  trashNote: "Cards in the Trash are deleted for good after 30 days.",
  emptyTrash: "Empty Trash",
  emptyTrashTitle: "Empty the Trash?",
  emptyTrashBody: (n: number) =>
    `${n} ${n === 1 ? "card" : "cards"} will be deleted for good. This can't be undone.`,
  deleteForeverTitle: "Delete this card for good?",
  deleteForeverBody: "It can't be restored afterwards.",
  deletedAgo: (days: number) =>
    days <= 0 ? "Deleted today" : `Deleted ${days} ${days === 1 ? "day" : "days"} ago`,
  archivedCards: "Cards",
  archivedProjects: "Projects",
  archivedBoards: "Boards",
  cancel: "Cancel",
  rename: "Rename",
  color: "Color",
  delete: "Delete",
  placeMenu: (name: string) => `Options for ${name}`,
  deleteProjectTitle: (name: string) => `Delete “${name}”?`,
  deleteProjectBody:
    "The project and its board are removed. Its cards move to the Trash, where you can restore them for 30 days.",
  deleteBoardTitle: (name: string) => `Delete “${name}”?`,
  deleteBoardBody:
    "The board is removed. Cards that live only on it move to the Trash for 30 days; project cards stay in their project.",
  placeArchived: (name: string) => `Archived “${name}”`,
  placeDeleted: (name: string, n: number) =>
    n > 0 ? `Deleted “${name}” · ${n} ${n === 1 ? "card" : "cards"} in Trash` : `Deleted “${name}”`,
} as const;

const workspaceCopy_en = {
  // Multi-select
  selected: (n: number) => `${n} selected`,
  bulkDone: "Done",
  bulkMove: "Move",
  bulkDate: "Date",
  bulkDelete: "Delete",
  bulkClear: "Clear selection",
  bulkDeleted: (n: number) => `Deleted ${n} ${n === 1 ? "card" : "cards"}`,
  bulkMoved: (n: number, place: string) => `Moved ${n} ${n === 1 ? "card" : "cards"} to ${place}`,
  moveTo: "Move to",
  inbox: "Inbox",
  setDate: "Set date",
  clearDate: "Clear date",
  // Undo / redo
  undone: (what: string) => `Undid ${what}`,
  redone: (what: string) => `Redid ${what}`,
  nothingToUndo: "Nothing to undo",
  nothingToRedo: "Nothing to redo",
  undo: "Undo",
  actions: {
    edit: "edit",
    complete: "complete",
    delete: "delete",
    move: "move",
    position: "move on canvas",
    group: "group change",
    bulk: "bulk change",
  },
  // Filters
  filter: "Filter",
  filters: "Filters",
  filterTitle: "Filter cards (⌘F)",
  filterText: "Search titles and notes…",
  filterTypes: "Type",
  filterStatus: "Status",
  filterProject: "Project",
  filterTag: "Tag",
  filterFrom: "From",
  filterTo: "To",
  anyProject: "Any project",
  noProject: "No project",
  anyTag: "Any tag",
  clearFilter: "Clear",
  closeFilter: "Close filters",
  saveFilter: "Save as filter",
  save: "Save",
  filterName: "Filter name",
  deleteFilter: (name: string) => `Delete filter ${name}`,
  matches: (n: number) => `${n} ${n === 1 ? "match" : "matches"}`,
  filterEmpty: {
    title: "Nothing matches this filter.",
    body: "Loosen a condition or clear the filter to see more cards.",
  },
  // Canvas groups
  groupTool: "Group",
  groupToolHint: "Drag on the canvas to draw a group",
  groupDefault: "Group",
  groupRename: "Rename group",
  groupDelete: "Delete group",
  // Graph
  viewGraph: "Graph",
  onlyLinked: "Only linked cards",
  graphEmpty: {
    title: "No cards to map.",
    body: "Connect cards on a board's canvas and they show up here as a map.",
  },
} as const;

const timeCopy_en = {
  // Calendar
  calendar: "Calendar",
  month: "Month",
  week: "Week",
  previous: "Previous",
  next: "Next",
  today: "Today",
  weekdays: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
  addOn: (day: string) => `Add a card on ${day}`,
  more: (n: number) => `+${n} more`,
  // Timeline
  timeline: "Timeline",
  nothingScheduled: "Nothing scheduled in these days. Give a card a date to see it here.",
  // Custom snooze
  later: "Later…",
  thisEvening: "This evening",
  nextMonday: "Next Monday",
  pickTime: "Pick a time",
  snooze: "Snooze",
  // Reminders before the due time
  due: (when: string) => `Due ${when}`,
  remindBefore: "Remind me before",
  remindBeforeNeedsDue: "Set a due date to be reminded before it",
  offsetLabel: (minutes: number) =>
    minutes % 10080 === 0
      ? `${minutes / 10080} ${minutes === 10080 ? "week" : "weeks"}`
      : minutes % 1440 === 0
        ? `${minutes / 1440} ${minutes === 1440 ? "day" : "days"}`
        : minutes % 60 === 0
          ? `${minutes / 60} ${minutes === 60 ? "hour" : "hours"}`
          : `${minutes} min`,
  // Habits
  habit: "Habit",
  habitHint: "Track a streak for this repeating card",
  habitNeedsRepeat: "Make the card repeat to track it as a habit",
  habits: "Habits",
  streak: (n: number) => `${n} in a row`,
  dayDone: "Done",
  dayMissed: "Not done",
  // Focus timer
  focus: "Focus",
  startFocus: (minutes: number) => `Focus ${minutes} min`,
  stopFocus: "Stop",
  focusing: "Focusing",
  focusLeft: (clock: string) => `${clock} left`,
  focusStats: (sessions: number, minutes: number) =>
    sessions === 0 && minutes === 0
      ? "No focus sessions yet"
      : `${sessions} ${sessions === 1 ? "session" : "sessions"} · ${minutes} min`,
  focusDone: "Focus session done",
  focusAgain: (minutes: number) => `${minutes} more`,
  markDone: "Mark done",
  focusOn: (title: string) => `Focus on ${title}`,
  // Evening recap
  recapTitle: (done: number, open: number) => `Your day · ${done} done, ${open} open`,
  recapDone: "Done today",
  recapOpen: "Still open",
  // Settings
  settingsTitle: "Time",
  recap: "Evening recap",
  recapHint: "After 19:00, a toast with what you finished today and what is still open.",
};

const voiceCopy_en = {
  // Toast confirming a voice command
  markedDone: "Marked done",
  movedTo: (when: string) => `Moved to ${when}`,
  moved: "Moved",
  // Card detail: the kept recording
  recording: "Recording",
  play: "Play recording",
  stop: "Stop",
  duration: (ms: number) => {
    const s = Math.max(1, Math.round(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  },
  // Settings → voice extras
  extrasTitle: "Voice commands & sounds",
  extrasLead:
    "Start with “mark … done”, “move … to tomorrow” or “what's on today?” and NOMA acts instead of taking a note. Anything unclear is saved as a note.",
  sound: "Confirmation sound",
  soundHint: "A soft chime when a voice capture is saved.",
  speak: "Answer aloud",
  speakHint: "“What's on today?” is answered with the Mac's voice, in your voice language.",
  keepAudio: "Keep the recording",
  keepAudioHint:
    "Saves what you said as audio next to the card, so you can play it back. Stays on this Mac.",
  vocabulary: "Words to recognize",
  vocabularyHint:
    "Names and words NOMA should expect, separated by commas. Project and board names are added for you.",
  vocabularyPlaceholder: "Marta, Kyoto, Figma",
};

/** Desktop presence extras: orb look, menu-bar count, quiet hours, panel search/capture, extra shortcuts. */
const desktopPresenceCopy_en = {
  menubarCount: "Count in the menu bar",
  menubarCountHint: "Shows how many things are left today next to the NOMA symbol.",
  orbSize: "Orb size",
  orbSizeHint: "How big the floating orb is.",
  orbSizes: { s: "S", m: "M", l: "L" },
  orbStyle: "Orb style",
  orbStyleHint:
    "Lively moves gently, Calm stays still, Faded is see-through until you point at it.",
  orbStyles: { lively: "Lively", calm: "Calm", faded: "Faded" },
  quiet: "Quiet hours",
  quietHint:
    'Every day in this window reminders wait and the orb stays still. Afterwards you\'ll see what came up as "While you were away".',
  quietFrom: "From",
  quietTo: "To",
  moreShortcuts: "More shortcuts",
  moreShortcutsHint: "Off until you set them, so they don't clash with your other apps.",
  shortcutToday: "Open Today",
  shortcutSearch: "Search",
  shortcutLastCapture: "Open last capture",
  shortcutNotSet: "Not set",
  shortcutClear: "Clear",
  orbAttention: "Something is due soon",
  orbQuiet: "Quiet hours",
  searchPlaceholder: "Search everything…",
  searchLabel: "Search",
  noResults: "Nothing found.",
  captureLabel: "Capture",
  capturePlaceholder: "Capture a thought…",
  captureSaved: "Saved to NOMA",
};

const dataCopy_en = {
  restoreTitle: "Restore a backup",
  restoreLead:
    "Go back to an earlier state of all your cards. NOMA backs up the current state first, so a restore can be undone too.",
  noSnapshots: "No backups yet.",
  snapshotSize: (bytes: number) =>
    bytes >= 1e6 ? `${(bytes / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1e3))} KB`,
  restore: "Restore",
  restoreConfirm: "Replace all your cards with this backup?",
  restoreYes: "Yes, restore",
  cancel: "Cancel",
  restoring: "Restoring…",
  restoreFailed: (msg: string) => `Couldn't restore: ${msg}`,

  importTitle: "Import",
  importLead:
    "Bring cards in from other apps. They land as they are, without AI sorting. Apple Notes and Reminders are only read, never changed.",
  importFrom: "From",
  importTo: "Into",
  inbox: "Inbox",
  sources: {
    markdown: "Markdown or text file",
    todoist: "Todoist CSV export",
    notes: "Apple Notes folder",
    reminders: "Apple Reminders list",
  },
  sourceHints: {
    markdown: "One card per list item or paragraph. Checkbox items become tasks.",
    todoist: "In Todoist: project menu → Export as CSV. One card per task.",
    notes: "One card per note in the folder you pick.",
    reminders: "One card per open reminder in the list you pick.",
  },
  chooseFile: "Choose file…",
  loadSources: (source: "notes" | "reminders") =>
    source === "notes" ? "Show folders" : "Show lists",
  pickSource: (source: "notes" | "reminders") =>
    source === "notes" ? "Choose a folder" : "Choose a list",
  importNow: "Import",
  importing: "Importing…",
  imported: (n: number, skipped: number) =>
    n === 0
      ? "Nothing to import."
      : `Imported ${n} card${n === 1 ? "" : "s"}.` +
        (skipped > 0 ? ` ${skipped} more were left out (limit reached).` : ""),
  importFailed: (msg: string) => `Import failed: ${msg}`,
  automationDenied:
    "NOMA isn't allowed to read this app. Allow it in System Settings → Privacy & Security → Automation, then try again.",

  autoExport: "Mirror to a folder",
  autoExportHint:
    "Keeps a readable noma.md with all your cards in a folder you choose, rewritten a few seconds after each change. Point it at an Obsidian vault or a synced folder.",
  noFolder: "No folder chosen",
  chooseFolder: "Choose folder…",
  folderPrompt: "Where should NOMA keep noma.md?",
  exportedTo: (path: string) => `Written to ${path}`,

  statsTitle: "Your rhythm",
  statsLead: "The last 8 weeks, counted on this Mac.",
  capturesLegend: "Captures",
  completedLegend: "Completed",
  avgLatency: (ms: number | null) =>
    ms === null ? "Average AI time: —" : `Average AI time: ${(ms / 1000).toFixed(1)} s`,
  totals: (captures: number, completed: number) => `${captures} captures · ${completed} completed`,
  weekOf: (date: string, captures: number, completed: number) =>
    `Week of ${date}: ${captures} captures, ${completed} completed`,
  chartLabel: "Captures and completed cards per week",

  historyTitle: "Capture history",
  historyLead: "What you said or typed, and what NOMA made of it.",
  noCaptures: "No captures yet.",
  status: {
    pending: "Waiting",
    processing: "Reading…",
    done: "Done",
    failed: "Failed",
  },
  source: { voice: "Voice", text: "Typed" },
  latency: (ms: number) => `${(ms / 1000).toFixed(1)} s`,
  becameCard: (title: string) => `→ ${title}`,
  cardGone: "→ card removed",
  retry: "Retry",
  showMore: "Show more",

  connectTitle: "Connect other apps",
  connectLead: "Capture into NOMA from scripts, the terminal, or other AI assistants on this Mac.",
  urlTitle: "Links",
  urlHint: "Open these from Shortcuts, Raycast, Alfred or a browser.",
  urlExamples: [
    { url: "noma://capture?text=Buy%20milk%20tomorrow", label: "Capture text" },
    { url: "noma://open?item=<card id>", label: "Show a card" },
    { url: "noma://today", label: "Show Today" },
  ],
  cliTitle: "Terminal command",
  cliHint: (path: string) =>
    `Installs a small "noma" script at ${path}. Make sure ~/.local/bin is on your PATH.`,
  cliUsage: 'noma buy milk tomorrow  ·  noma today  ·  echo "…" | noma',
  cliInstall: "Install command",
  cliReinstall: "Reinstall",
  cliInstalled: "Installed.",
  showScript: "Show script",
  copy: "Copy",
  copied: "Copied",
  mcpTitle: "MCP server",
  mcpHint:
    "Lets assistants such as Claude Desktop search your cards, read Today and add captures. Add this to the assistant's MCP settings:",
  mcpMissing:
    "The noma-mcp program isn't next to NOMA yet. In a development build, run: cargo build --bin noma-mcp",
  mcpCaptureNote: "Captures from assistants are picked up within about 30 seconds while NOMA runs.",
} as const;

/** Strings that used to live inline: region labels, palette commands, status chips, toasts. */
const uiCopy_en = {
  regions: {
    settings: "Settings",
    workspace: "Workspace",
    mainNav: "Main",
    cardDetails: "Card details",
    commandPalette: "Command palette",
    quickAdd: "Quick add",
    viewMode: "View mode",
    cards: "Cards",
    quickCapture: "Quick capture",
    panel: "NOMA panel",
    reminder: "Reminder",
    missedReminders: "Missed reminders",
    today: "Today",
    captureFiled: "Capture filed",
    voiceCapture: "Voice capture",
    editTranscript: "Edit transcript",
    type: "Type",
  },
  dismiss: "Dismiss",
  undo: "Undo",
  retry: "Retry",
  close: "Close",
  add: "Add",
  duplicate: "Duplicate",
  delete: "Delete",
  captureFailed: "Couldn't read that capture. It's kept as a note.",
  recurringDone: "Done for now — next one scheduled.",
  removeTag: (tag: string) => `Remove ${tag}`,
  zoomIn: "Zoom in",
  zoomOut: "Zoom out",
  needsCheck: "Needs a quick check",
  ai: {
    captured: "Captured",
    idle: "Idle",
    downloading: (pct: number) => `Downloading ${pct}%`,
    wakingUp: "Waking up",
    thinking: "Thinking",
    needsAttention: "Needs attention",
    waiting: (n: number) => `${n} waiting`,
    setUp: "Set up AI",
  },
  palette: {
    goTo: (place: string) => `Go to ${place}`,
    newKind: {
      task: "New task",
      reminder: "New reminder",
      note: "New note",
      idea: "New idea",
      project_ref: "New project",
      event: "New event",
    },
    canvasView: "Canvas view",
    kanbanView: "Kanban view",
    listView: "List view",
    toggleSidebar: "Toggle sidebar",
    themeDark: "Theme: Dark",
    themeLight: "Theme: Light",
    themeSystem: "Theme: Match system",
    languageEnglish: "Language: English",
    languageItalian: "Language: Italiano",
    languageSystem: "Language: Match system",
    loadSample: "Load sample data",
    sampleLoaded: "Sample data loaded.",
    sampleNeedsEmpty: "Sample data needs an empty database.",
  },
} as const;

/** Words used by the date helpers in lib/dates.ts. */
const dateCopy_en = {
  /** BCP 47 locale for weekday and time formatting. */
  intl: "en-GB",
  /** Locale for short month names ("Apr"). */
  intlMonth: "en-US",
  today: "Today",
  tomorrow: "Tomorrow",
  yesterday: "Yesterday",
  weekdayDay: (weekday: string, day: number) => `${weekday} ${day}`,
  monthDay: (month: string, day: number) => `${month} ${day}`,
  monthDayYear: (month: string, day: number, year: number) => `${month} ${day}, ${year}`,
  units: { m: "m", h: "h", d: "d" },
  now: "now",
  inFuture: (span: string) => `in ${span}`,
  inPast: (span: string) => `${span} ago`,
  editedJustNow: "edited just now",
  editedAgo: (span: string) => `edited ${span} ago`,
  editedYesterday: "edited yesterday",
  editedOn: (day: string) => `edited ${day}`,
};

function ordinal(n: number): string {
  const s = ["th", "st", "nd", "rd"];
  const v = n % 100;
  return n + (s[(v - 20) % 10] ?? s[v] ?? s[0]!);
}

/** Repeat presets and plain-language descriptions of repeat rules (lib/recurrence.ts). */
const recurrenceCopy_en = {
  presets: {
    none: "Doesn't repeat",
    daily: "Every day",
    weekdays: "Every weekday",
    weekly: "Every week",
    monthly: "Every month",
    yearly: "Every year",
  },
  days: {
    MO: "Monday",
    TU: "Tuesday",
    WE: "Wednesday",
    TH: "Thursday",
    FR: "Friday",
    SA: "Saturday",
    SU: "Sunday",
  } as Record<string, string>,
  every: (unit: "day" | "week" | "month" | "year", n: number) =>
    n > 1 ? `every ${n} ${unit}s` : `every ${unit}`,
  everyWeekday: "every weekday",
  everyDay: (day: string) => `every ${day}`,
  onDays: (every: string, days: string) => `${every} on ${days}`,
  onMonthDay: (every: string, n: number) => `${every} on the ${ordinal(n)}`,
  times: (n: number) => `, ${n} times`,
  repeats: "repeats",
};

// ── Locales ─────────────────────────────────────────────────────────────

const en = {
  copy: copy_en,
  kindLabel: kindLabel_en,
  statusLabel: statusLabel_en,
  priorityLabel: priorityLabel_en,
  cardCopy: cardCopy_en,
  listCopy: listCopy_en,
  quickAddCopy: quickAddCopy_en,
  hintCopy: hintCopy_en,
  settingsCopy: settingsCopy_en,
  detailCopy: detailCopy_en,
  paletteCopy: paletteCopy_en,
  sidebarCopy: sidebarCopy_en,
  desktopCopy: desktopCopy_en,
  onboardingCopy: onboardingCopy_en,
  organizeCopy: organizeCopy_en,
  semanticCopy: semanticCopy_en,
  cardsCopy: cardsCopy_en,
  workspaceCopy: workspaceCopy_en,
  timeCopy: timeCopy_en,
  voiceCopy: voiceCopy_en,
  desktopPresenceCopy: desktopPresenceCopy_en,
  dataCopy: dataCopy_en,
  uiCopy: uiCopy_en,
  dateCopy: dateCopy_en,
  recurrenceCopy: recurrenceCopy_en,
};

/** Same shape as the English strings, with every literal widened to `string`. */
type Widen<T> = T extends string
  ? string
  : T extends (...args: infer A) => infer R
    ? (...args: A) => Widen<R>
    : T extends object
      ? { readonly [K in keyof T]: Widen<T[K]> }
      : T;

/** Every string in the app, for one language. `copy.it.ts` implements it in Italian. */
export type CopyBundle = Widen<typeof en>;

const bundles: Record<Locale, CopyBundle> = { en, it };

/**
 * A live view of one section: property reads go to the active language, so
 * `copy.nav.inbox` follows the interface language without changing call sites.
 * Read strings at render time (not at module load) so they follow a switch.
 */
function live<K extends keyof CopyBundle>(key: K): CopyBundle[K] {
  const pick = () => bundles[getLocale()][key] as object;
  return new Proxy(en[key] as object, {
    get: (_t, p) => Reflect.get(pick(), p),
    has: (_t, p) => Reflect.has(pick(), p),
    ownKeys: () => Reflect.ownKeys(pick()),
    getOwnPropertyDescriptor: (_t, p) => {
      const d = Reflect.getOwnPropertyDescriptor(pick(), p);
      return d && { ...d, configurable: true };
    },
  }) as CopyBundle[K];
}

export const copy = live("copy");
export const kindLabel = live("kindLabel");
export const statusLabel = live("statusLabel");
export const priorityLabel = live("priorityLabel");
export const cardCopy = live("cardCopy");
export const listCopy = live("listCopy");
export const quickAddCopy = live("quickAddCopy");
export const hintCopy = live("hintCopy");
export const settingsCopy = live("settingsCopy");
export const detailCopy = live("detailCopy");
export const paletteCopy = live("paletteCopy");
export const sidebarCopy = live("sidebarCopy");
export const desktopCopy = live("desktopCopy");
export const onboardingCopy = live("onboardingCopy");
export const organizeCopy = live("organizeCopy");
export const semanticCopy = live("semanticCopy");
export const cardsCopy = live("cardsCopy");
export const workspaceCopy = live("workspaceCopy");
export const timeCopy = live("timeCopy");
export const voiceCopy = live("voiceCopy");
export const desktopPresenceCopy = live("desktopPresenceCopy");
export const dataCopy = live("dataCopy");
export const uiCopy = live("uiCopy");
export const dateCopy = live("dateCopy");
export const recurrenceCopy = live("recurrenceCopy");

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
  orbHint: "⌥ Space to talk",
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
      body: "Coming with the next phase.",
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
  hint: "↵ to add · Esc to close",
  add: "Add",
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

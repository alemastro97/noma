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

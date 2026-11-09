import { Bell, CalendarDays, CheckSquare, FileText, Folder, Lightbulb } from "lucide-react";
import type { ComponentType } from "react";
import type { ItemType } from "../../api/types";

interface KindStyle {
  Icon: ComponentType<{ size?: number; strokeWidth?: number; "aria-hidden"?: boolean }>;
  /** Accent color (CSS var) used for the icon, label and checkbox ring. */
  color: string;
}

export const KIND_STYLE: Record<ItemType, KindStyle> = {
  task: { Icon: CheckSquare, color: "var(--noma-task)" },
  reminder: { Icon: Bell, color: "var(--noma-reminder)" },
  note: { Icon: FileText, color: "var(--noma-text-2)" },
  idea: { Icon: Lightbulb, color: "var(--noma-idea)" },
  project_ref: { Icon: Folder, color: "var(--noma-project)" },
  event: { Icon: CalendarDays, color: "var(--noma-project)" },
};

import { Fragment, type ReactNode } from "react";
import { useLocale } from "./locale";

/**
 * Re-mounts its children when the interface language changes, so every string
 * read from `copy.ts` (including memoized ones) is read again in the new language.
 * Stores live outside React, so data and navigation survive the switch.
 */
export function LocaleRoot({ children }: { children: ReactNode }) {
  const locale = useLocale();
  return <Fragment key={locale}>{children}</Fragment>;
}

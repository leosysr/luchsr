/**
 * Theme-Umschaltung.
 *
 * Ohne [data-theme] auf <html> folgt die App dem Systemmodus — das regelt
 * `color-scheme: light dark` in tokens.css zusammen mit light-dark().
 * Mit [data-theme] wird der Modus festgenagelt.
 *
 * Hier steht bewusst KEIN Farbwert: diese Datei setzt nur ein Attribut.
 */

export type ThemePreference = "system" | "light" | "dark";

export const THEME_PREFERENCES: readonly ThemePreference[] = [
  "system",
  "light",
  "dark",
] as const;

/** Setzt bzw. entfernt das Override-Attribut auf <html>. */
export function applyTheme(preference: ThemePreference): void {
  const root = document.documentElement;
  if (preference === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", preference);
  }
}

/** Liest zurück, was aktuell gesetzt ist. */
export function currentPreference(): ThemePreference {
  const value = document.documentElement.getAttribute("data-theme");
  return value === "light" || value === "dark" ? value : "system";
}

/**
 * Welcher Modus tatsächlich greift — inklusive aufgelöstem Systemmodus.
 * Nützlich, um dem Rust-Backend das passende Tray-Icon zu nennen.
 */
export function effectiveTheme(): "light" | "dark" {
  const preference = currentPreference();
  if (preference !== "system") return preference;
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/**
 * Benachrichtigt, wenn sich der Systemmodus ändert — greift nur, solange
 * kein manueller Override gesetzt ist.
 */
export function onSystemThemeChange(
  handler: (theme: "light" | "dark") => void,
): () => void {
  const query = window.matchMedia("(prefers-color-scheme: dark)");
  const listener = (event: MediaQueryListEvent) => {
    if (currentPreference() === "system") {
      handler(event.matches ? "dark" : "light");
    }
  };
  query.addEventListener("change", listener);
  return () => query.removeEventListener("change", listener);
}

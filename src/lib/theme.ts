export function applyTheme(theme: "system" | "light" | "dark"): () => void {
  const root = document.documentElement;
  const media = window.matchMedia("(prefers-color-scheme: dark)");

  const update = () => {
    const isDark =
      theme === "dark" || (theme === "system" && media.matches);
    root.classList.toggle("dark", isDark);
  };

  update();

  if (theme === "system") {
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }
  return () => {};
}

export function formatCountdown(totalSecs: number): string {
  const clamped = Math.max(0, Math.floor(totalSecs));
  const mins = Math.floor(clamped / 60);
  const secs = clamped % 60;
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

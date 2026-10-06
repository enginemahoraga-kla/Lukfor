// Shut down / restart / sleep rows. Typing the word only offers the action;
// it takes a first Enter to arm it and a second, separate Enter to run it.

export type PowerAction = "shutdown" | "restart" | "sleep";

const WORDS: Record<PowerAction, string[]> = {
  shutdown: ["shutdown", "shut down", "power off", "matikan"],
  restart: ["restart", "reboot", "mulai ulang"],
  sleep: ["sleep", "tidur"],
};

export const POWER_TEXT: Record<PowerAction, { label: string; ask: string }> = {
  shutdown: { label: "Shut down", ask: "Shut down now?" },
  restart: { label: "Restart", ask: "Restart now?" },
  sleep: { label: "Sleep", ask: "Sleep now?" },
};

/** An armed action disarms itself after this long without a confirmation. */
export const ARM_FOR_MS = 6000;

/**
 * A confirmation arriving sooner than this after arming is ignored, so a
 * double-click or a held-down Enter can't do both steps in one motion.
 */
export const CONFIRM_AFTER_MS = 600;

/** Actions whose words start with the query (3+ characters, like "res"). */
export function powerMatches(query: string): PowerAction[] {
  const q = query.trim().toLowerCase();
  if (q.length < 3) return [];
  return (Object.keys(WORDS) as PowerAction[]).filter((a) => WORDS[a].some((w) => w.startsWith(q)));
}

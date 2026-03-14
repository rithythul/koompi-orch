type KeyHandler = () => void;

interface Keybinding {
  key: string;
  mod: ("ctrl" | "meta" | "shift" | "alt")[];
  handler: KeyHandler;
  description: string;
}

const bindings: Keybinding[] = [];
const isMac = typeof navigator !== "undefined" && navigator.platform.startsWith("Mac");

function matchesMod(e: KeyboardEvent, mods: string[]): boolean {
  const needCtrl = mods.includes("ctrl");
  const needMeta = mods.includes("meta");
  const needShift = mods.includes("shift");
  const needAlt = mods.includes("alt");

  // "ctrl" maps to Cmd on Mac, Ctrl elsewhere
  const modKey = isMac ? e.metaKey : e.ctrlKey;

  return (
    (needCtrl ? modKey : !modKey || needMeta) &&
    (needMeta ? e.metaKey : true) &&
    (needShift ? e.shiftKey : !e.shiftKey) &&
    (needAlt ? e.altKey : !e.altKey)
  );
}

function handleKeydown(e: KeyboardEvent) {
  for (const binding of bindings) {
    if (e.key.toLowerCase() === binding.key.toLowerCase() && matchesMod(e, binding.mod)) {
      e.preventDefault();
      binding.handler();
      return;
    }
  }
}

// Track listener to avoid duplicates in StrictMode
let listenerAttached = false;

export function registerKeybinding(
  key: string,
  mod: ("ctrl" | "meta" | "shift" | "alt")[],
  handler: KeyHandler,
  description: string,
) {
  // Dedup: remove existing binding for same key+mod combo
  const idx = bindings.findIndex(
    (b) => b.key === key && JSON.stringify(b.mod) === JSON.stringify(mod),
  );
  if (idx !== -1) bindings.splice(idx, 1);

  bindings.push({ key, mod, handler, description });

  if (!listenerAttached) {
    document.addEventListener("keydown", handleKeydown);
    listenerAttached = true;
  }
}

export function unregisterAllKeybindings() {
  bindings.length = 0;
  document.removeEventListener("keydown", handleKeydown);
  listenerAttached = false;
}

export function getKeybindings(): readonly Keybinding[] {
  return bindings;
}

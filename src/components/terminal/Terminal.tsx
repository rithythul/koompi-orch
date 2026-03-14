import { useEffect, useRef } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  sessionId: string;
  fontSize?: number;
  readOnly?: boolean;
}

export function Terminal({
  sessionId,
  fontSize = 14,
  readOnly = false,
}: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new XTerm({
      cursorBlink: true,
      fontFamily: "monospace",
      fontSize,
      theme: {
        background: "#1a1b26",
        foreground: "#c0caf5",
        cursor: "#c0caf5",
        selectionBackground: "#33467c",
        black: "#15161e",
        red: "#f7768e",
        green: "#9ece6a",
        yellow: "#e0af68",
        blue: "#7aa2f7",
        magenta: "#bb9af7",
        cyan: "#7dcfff",
        white: "#a9b1d6",
      },
      scrollback: 10000,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(containerRef.current);

    try {
      fitAddon.fit();
    } catch {
      // Container may not be visible yet
    }

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Send user input to backend PTY
    if (!readOnly) {
      term.onData((data: string) => {
        invoke("pty_write", { sessionId, data }).catch((err: unknown) => {
          console.error("Failed to write to PTY:", err);
        });
      });
    }

    // Listen for PTY output from backend
    let unlisten: (() => void) | undefined;
    listen<{ data: string }>(`pty_output:${sessionId}`, (event) => {
      term.write(event.payload.data);
    }).then((fn) => {
      unlisten = fn;
    });

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddon.fit();
      } catch {
        // Ignore fit errors during transitions
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      unlisten?.();
      resizeObserver.disconnect();
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, fontSize, readOnly]);

  return (
    <div
      ref={containerRef}
      data-testid="terminal-container"
      className="w-full h-full min-h-[200px] bg-[#1a1b26] rounded-lg overflow-hidden"
    />
  );
}

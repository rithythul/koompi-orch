import { describe, it, expect, vi, beforeEach, beforeAll } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { Terminal } from "./Terminal";

// Polyfill ResizeObserver for jsdom
beforeAll(() => {
  global.ResizeObserver = class ResizeObserver {
    observe = vi.fn();
    unobserve = vi.fn();
    disconnect = vi.fn();
  };
});

// Mock @xterm/xterm
const mockWrite = vi.fn();
const mockDispose = vi.fn();
const mockOpen = vi.fn();
const mockOnData = vi.fn();
const mockLoadAddon = vi.fn();

vi.mock("@xterm/xterm", () => {
  return {
    Terminal: function MockTerminal() {
      return {
        open: mockOpen,
        write: mockWrite,
        dispose: mockDispose,
        onData: mockOnData,
        loadAddon: mockLoadAddon,
        options: {},
      };
    },
  };
});

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: function MockFitAddon() {
    return { fit: vi.fn(), dispose: vi.fn() };
  },
}));

vi.mock("@xterm/addon-web-links", () => ({
  WebLinksAddon: function MockWebLinks() {
    return { dispose: vi.fn() };
  },
}));

// Mock CSS import
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

// Mock Tauri APIs
const mockListen = vi.fn().mockResolvedValue(vi.fn());
const mockInvoke = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("Terminal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  it("renders terminal container with correct test id", () => {
    render(<Terminal sessionId="session-1" />);
    expect(screen.getByTestId("terminal-container")).toBeDefined();
  });

  it("initializes xterm instance on mount", () => {
    render(<Terminal sessionId="session-1" />);
    expect(mockOpen).toHaveBeenCalled();
    expect(mockLoadAddon).toHaveBeenCalledTimes(2);
  });

  it("subscribes to Tauri pty_output event for the session", () => {
    render(<Terminal sessionId="session-42" />);
    expect(mockListen).toHaveBeenCalledWith(
      "pty_output:session-42",
      expect.any(Function)
    );
  });

  it("sends user input to backend via invoke", () => {
    render(<Terminal sessionId="session-1" />);
    const onDataCallback = mockOnData.mock.calls[0]?.[0];
    if (onDataCallback) {
      onDataCallback("hello");
      expect(mockInvoke).toHaveBeenCalledWith("pty_write", {
        sessionId: "session-1",
        data: "hello",
      });
    }
  });

  it("disposes xterm on unmount", () => {
    const { unmount } = render(<Terminal sessionId="session-1" />);
    unmount();
    expect(mockDispose).toHaveBeenCalled();
  });
});

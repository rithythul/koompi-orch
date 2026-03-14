import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, cleanup, waitFor } from "@testing-library/react";
import { useLiveQuery } from "./useLiveQuery";

type ListenerCallback = (event: { payload: unknown }) => void;
const listeners = new Map<string, ListenerCallback>();
const mockUnlisten = vi.fn();

const mockListen = vi.fn((event: string, callback: ListenerCallback) => {
  listeners.set(event, callback);
  return Promise.resolve(mockUnlisten);
});

const mockInvoke = vi.fn().mockResolvedValue("live-query-uuid-123");

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...(args as Parameters<typeof mockListen>)),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("useLiveQuery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
  });

  afterEach(() => {
    cleanup();
  });

  it("returns initial empty data and loading true", () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [],
      })
    );
    expect(result.current.data).toEqual([]);
    expect(result.current.loading).toBe(true);
  });

  it("registers a Tauri event listener for the table", async () => {
    renderHook(() =>
      useLiveQuery<{ id: string }>({ table: "workspace", initialData: [] })
    );
    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith(
        "live:workspace",
        expect.any(Function)
      );
    });
  });

  it("invokes start_live_query on mount", async () => {
    renderHook(() =>
      useLiveQuery<{ id: string }>({ table: "session", initialData: [] })
    );
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("start_live_query", {
        table: "session",
      });
    });
  });

  it("updates data on CREATE event", async () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [],
      })
    );

    await waitFor(() => {
      expect(listeners.get("live:workspace")).toBeDefined();
    });

    const callback = listeners.get("live:workspace")!;

    act(() => {
      callback({
        payload: {
          action: "CREATE",
          result: { id: "ws-1", name: "feat-auth" },
        },
      });
    });

    expect(result.current.data).toEqual([{ id: "ws-1", name: "feat-auth" }]);
  });

  it("updates existing record on UPDATE event", async () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [{ id: "ws-1", name: "old-name" }],
      })
    );

    await waitFor(() => {
      expect(listeners.get("live:workspace")).toBeDefined();
    });

    const callback = listeners.get("live:workspace")!;
    act(() => {
      callback({
        payload: {
          action: "UPDATE",
          result: { id: "ws-1", name: "new-name" },
        },
      });
    });

    expect(result.current.data).toEqual([{ id: "ws-1", name: "new-name" }]);
  });

  it("removes record on DELETE event", async () => {
    const { result } = renderHook(() =>
      useLiveQuery<{ id: string; name: string }>({
        table: "workspace",
        initialData: [
          { id: "ws-1", name: "keep" },
          { id: "ws-2", name: "remove" },
        ],
      })
    );

    await waitFor(() => {
      expect(listeners.get("live:workspace")).toBeDefined();
    });

    const callback = listeners.get("live:workspace")!;
    act(() => {
      callback({
        payload: {
          action: "DELETE",
          result: { id: "ws-2", name: "remove" },
        },
      });
    });

    expect(result.current.data).toEqual([{ id: "ws-1", name: "keep" }]);
  });

  it("unlistens and stops query on unmount", async () => {
    const { unmount } = renderHook(() =>
      useLiveQuery<{ id: string }>({ table: "workspace", initialData: [] })
    );

    // Wait for setup to complete
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("start_live_query", {
        table: "workspace",
      });
    });

    unmount();

    expect(mockUnlisten).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("stop_live_query", {
      queryId: "live-query-uuid-123",
    });
  });
});

import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface LiveQueryAction<T> {
  action: "CREATE" | "UPDATE" | "DELETE";
  result: T;
}

interface UseLiveQueryOptions<T extends { id: string }> {
  table: string;
  initialData: T[];
  filter?: (record: T) => boolean;
  enabled?: boolean;
}

interface UseLiveQueryResult<T> {
  data: T[];
  loading: boolean;
  error: string | null;
}

export function useLiveQuery<T extends { id: string }>({
  table,
  initialData,
  filter,
  enabled = true,
}: UseLiveQueryOptions<T>): UseLiveQueryResult<T> {
  const [data, setData] = useState<T[]>(initialData);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const queryIdRef = useRef<string | null>(null);

  const handleEvent = useCallback(
    (event: { payload: unknown }) => {
      const { action, result } = event.payload as LiveQueryAction<T>;
      if (filter && !filter(result)) return;

      setData((prev) => {
        switch (action) {
          case "CREATE":
            if (prev.some((item) => item.id === result.id)) {
              return prev.map((item) =>
                item.id === result.id ? result : item
              );
            }
            return [...prev, result];

          case "UPDATE":
            return prev.map((item) =>
              item.id === result.id ? result : item
            );

          case "DELETE":
            return prev.filter((item) => item.id !== result.id);

          default:
            return prev;
        }
      });

      setLoading(false);
    },
    [filter]
  );

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    const setup = async () => {
      try {
        unlisten = await listen(`live:${table}`, handleEvent);

        const queryId = await invoke<string>("start_live_query", { table });
        if (!cancelled) {
          queryIdRef.current = queryId;
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
          setLoading(false);
        }
      }
    };

    setup();

    return () => {
      cancelled = true;
      unlisten?.();

      if (queryIdRef.current) {
        invoke("stop_live_query", { queryId: queryIdRef.current }).catch(
          (err: unknown) =>
            console.error("Failed to stop live query:", err)
        );
        queryIdRef.current = null;
      }
    };
  }, [table, enabled, handleEvent]);

  return { data, loading, error };
}

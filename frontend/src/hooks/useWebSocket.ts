"use client";

import { useEffect, useRef } from "react";

type EventHandlers = Record<string, (data: unknown) => void>;

const WS_BASE = (process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:7878")
  .replace(/^http/, "ws");

export function useWebSocket(token: string | null, handlers: EventHandlers) {
  const handlersRef = useRef<EventHandlers>(handlers);

  useEffect(() => {
    handlersRef.current = handlers;
  });

  useEffect(() => {
    if (!token) return;

    let ws: WebSocket;
    let reconnectTimer: ReturnType<typeof setTimeout>;
    let alive = true;

    function connect() {
      if (!alive) return;
      ws = new WebSocket(`${WS_BASE}/ws?token=${token}`);

      ws.onmessage = (ev) => {
        try {
          const { event, data } = JSON.parse(ev.data as string);
          handlersRef.current[event]?.(data);
        } catch {}
      };

      ws.onclose = () => {
        if (alive) reconnectTimer = setTimeout(connect, 3000);
      };

      ws.onerror = () => ws.close();
    }

    connect();

    return () => {
      alive = false;
      clearTimeout(reconnectTimer);
      ws?.close();
    };
  }, [token]);
}

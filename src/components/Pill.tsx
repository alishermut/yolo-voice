import { useState, useEffect, useRef } from "react";
import type { PillState } from "../shared/types";
import { onRecordingState, onRecordingLevel } from "../shared/platform";

function Waveform({ level, barCount = 9 }: { level: number; barCount?: number }) {
  const [bars, setBars] = useState<number[]>(Array(barCount).fill(8));
  const levelRef = useRef(level);
  levelRef.current = level;

  useEffect(() => {
    const interval = setInterval(() => {
      const l = levelRef.current;
      setBars(prev =>
        prev.map(() => {
          if (l < 2) return 8;
          const base = l * 0.9;
          const variation = (Math.random() - 0.5) * l * 0.5;
          return Math.max(8, Math.min(100, base + variation));
        })
      );
    }, 80);
    return () => clearInterval(interval);
  }, []);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: "2px", height: "22px" }}>
      {bars.map((h, i) => (
        <div
          key={i}
          style={{
            width: "3px",
            borderRadius: "999px",
            transition: "height 80ms ease",
            height: `${h}%`,
            backgroundColor: `rgba(74, 222, 128, ${0.5 + h / 200})`,
          }}
        />
      ))}
    </div>
  );
}

export function Pill() {
  const [state, setState] = useState<PillState>("idle");
  const [level, setLevel] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const doneTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const transcribeStart = useRef<number | null>(null);

  // Elapsed timer for transcribing state
  useEffect(() => {
    if (state === "transcribing") {
      if (transcribeStart.current === null) {
        transcribeStart.current = Date.now();
      }
      const timer = setInterval(() => {
        setElapsed(Math.floor((Date.now() - (transcribeStart.current ?? Date.now())) / 1000));
      }, 200);
      return () => clearInterval(timer);
    } else {
      transcribeStart.current = null;
      setElapsed(0);
    }
  }, [state]);

  // Event listeners
  useEffect(() => {
    const unlistenState = onRecordingState((newState) => {
      if (newState === "done") {
        setState("done");
        if (doneTimer.current) clearTimeout(doneTimer.current);
        doneTimer.current = setTimeout(() => setState("idle"), 1200);
      } else {
        setState(newState);
        if (doneTimer.current) {
          clearTimeout(doneTimer.current);
          doneTimer.current = null;
        }
      }
    });

    const unlistenLevel = onRecordingLevel((audioLevel) => {
      setLevel(audioLevel);
    });

    return () => {
      unlistenState.then((fn) => fn());
      unlistenLevel.then((fn) => fn());
    };
  }, []);

  const isActive = state !== "idle";

  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "flex-end",
        justifyContent: "center",
        paddingBottom: "4px",
        background: "transparent",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: isActive ? "8px" : "0",
          background: "rgba(20, 20, 28, 0.95)",
          borderRadius: "999px",
          padding: isActive ? "7px 16px" : "0",
          width: isActive ? "auto" : "40px",
          height: isActive ? "auto" : "6px",
          minHeight: isActive ? "34px" : "6px",
          border: `1px solid ${
            state === "recording"
              ? "rgba(74, 222, 128, 0.4)"
              : state === "transcribing"
                ? "rgba(59, 130, 246, 0.4)"
                : state === "done"
                  ? "rgba(74, 222, 128, 0.4)"
                  : "rgba(60, 60, 70, 0.4)"
          }`,
          boxShadow:
            state === "recording"
              ? "0 0 12px rgba(74, 222, 128, 0.15)"
              : state === "transcribing"
                ? "0 0 12px rgba(59, 130, 246, 0.15)"
                : "none",
          transition: "all 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
          overflow: "hidden",
        }}
      >
        {/* RECORDING */}
        {state === "recording" && (
          <>
            <div style={{ position: "relative", width: "10px", height: "10px", flexShrink: 0 }}>
              <div
                style={{
                  position: "absolute",
                  inset: "-4px",
                  borderRadius: "50%",
                  background: "rgba(74, 222, 128, 0.25)",
                  animation: "ping 1.5s cubic-bezier(0,0,0.2,1) infinite",
                }}
              />
              <div style={{ width: "10px", height: "10px", borderRadius: "50%", background: "#4ade80" }} />
            </div>
            <Waveform level={level} barCount={9} />
            <span style={{ color: "rgba(74, 222, 128, 0.7)", fontSize: "10px", fontWeight: 600, letterSpacing: "1px" }}>
              REC
            </span>
          </>
        )}

        {/* TRANSCRIBING */}
        {state === "transcribing" && (
          <>
            <div
              style={{
                width: "12px",
                height: "12px",
                borderRadius: "50%",
                border: "2px solid rgba(59, 130, 246, 0.3)",
                borderTopColor: "#3b82f6",
                animation: "spin 0.8s linear infinite",
                flexShrink: 0,
              }}
            />
            <span style={{ color: "rgba(147, 197, 253, 0.8)", fontSize: "10px", fontWeight: 500 }}>
              {elapsed > 0 ? `${elapsed}s` : "Processing..."}
            </span>
          </>
        )}

        {/* DONE */}
        {state === "done" && (
          <>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#4ade80" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
              <path d="M5 13l4 4L19 7" />
            </svg>
            <span style={{ color: "#4ade80", fontSize: "10px", fontWeight: 500 }}>Done</span>
          </>
        )}
      </div>

      <style>{`
        @keyframes ping { 75%, 100% { transform: scale(2.5); opacity: 0; } }
        @keyframes spin { to { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
}

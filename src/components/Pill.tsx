import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type { ActiveMode, PillState } from "../shared/types";
import { onRecordingState, onRecordingLevel, onActiveMode, onStyleSwitched, getConfig } from "../shared/platform";

function Waveform({ level, barCount = 9, color }: { level: number; barCount?: number; color: string }) {
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
            backgroundColor: color.replace("1)", `${0.5 + h / 200})`),
          }}
        />
      ))}
    </div>
  );
}

const COLORS = {
  dictation: {
    accent: "rgba(74, 222, 128,",
    border: "rgba(74, 222, 128, 0.4)",
    glow: "rgba(74, 222, 128, 0.15)",
    text: "rgba(74, 222, 128, 0.7)",
    solid: "#4ade80",
  },
  command: {
    accent: "rgba(168, 85, 247,",
    border: "rgba(168, 85, 247, 0.4)",
    glow: "rgba(168, 85, 247, 0.15)",
    text: "rgba(168, 85, 247, 0.7)",
    solid: "#a855f7",
  },
};

const PILL_SIZE = new LogicalSize(280, 50);

export function Pill() {
  const { t } = useTranslation();
  const [state, setState] = useState<PillState>("idle");
  const [mode, setMode] = useState<ActiveMode>("dictation");
  const [level, setLevel] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const [styleName, setStyleName] = useState<string | null>(null);
  const [pinned, setPinned] = useState(false);
  const doneTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const styleTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const transcribeStart = useRef<number | null>(null);
  const windowReady = useRef(false);

  // Load pinned state from config
  useEffect(() => {
    getConfig()
      .then((config) => setPinned(config.pill_pinned ?? false))
      .catch(() => {});

    const unlisten = listen<boolean>("pill-pinned-changed", (event) => {
      setPinned(event.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  // Window visibility — keep window always shown at full size to avoid
  // show/hide delay. Use ignore_cursor_events so the transparent area
  // doesn't block clicks when idle.
  useEffect(() => {
    const win = getCurrentWindow();
    const isActive = state !== "idle";
    const shouldBeVisible = isActive || pinned;

    if (shouldBeVisible && !windowReady.current) {
      // First time becoming visible: set size and show
      win.setSize(PILL_SIZE).catch(() => {});
      win.show().catch(() => {});
      win.setAlwaysOnTop(true).catch(() => {});
      win.setIgnoreCursorEvents(false).catch(() => {});
      windowReady.current = true;
    } else if (shouldBeVisible) {
      // Already visible — just ensure interactive
      win.setIgnoreCursorEvents(false).catch(() => {});
    } else if (windowReady.current) {
      // Going invisible: hide the window
      win.hide().catch(() => {});
      windowReady.current = false;
    }
  }, [state, pinned]);

  // Pre-show the window on recording start to eliminate delay.
  // When we go from hidden → recording, show + resize in one shot.
  useEffect(() => {
    if (state === "recording" && !windowReady.current) {
      const win = getCurrentWindow();
      win.setSize(PILL_SIZE).catch(() => {});
      win.show().catch(() => {});
      win.setAlwaysOnTop(true).catch(() => {});
      win.setIgnoreCursorEvents(false).catch(() => {});
      windowReady.current = true;
    }
  }, [state]);

  // Elapsed timer
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
        setStyleName(null);
        if (doneTimer.current) clearTimeout(doneTimer.current);
        doneTimer.current = setTimeout(() => {
          setState("idle");
          setMode("dictation");
        }, 1200);
      } else if (newState === "idle") {
        setState("idle");
        setMode("dictation");
        setStyleName(null);
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

    const unlistenMode = onActiveMode((newMode) => {
      setMode(newMode);
    });

    const unlistenStyle = onStyleSwitched((name) => {
      setStyleName(name);
      if (styleTimer.current) clearTimeout(styleTimer.current);
    });

    return () => {
      unlistenState.then((fn) => fn());
      unlistenLevel.then((fn) => fn());
      unlistenMode.then((fn) => fn());
      unlistenStyle.then((fn) => fn());
    };
  }, []);

  const isActive = state !== "idle";
  const isCommand = mode === "command";
  const colors = isCommand ? COLORS.command : COLORS.dictation;
  const label = isCommand ? t("pill.recording.labelCommand") : t("pill.recording.labelDictation");

  const transcribingColor = isCommand
    ? { border: "rgba(168, 85, 247, 0.3)", top: "#a855f7", text: "rgba(216, 180, 254, 0.8)" }
    : { border: "rgba(59, 130, 246, 0.3)", top: "#3b82f6", text: "rgba(147, 197, 253, 0.8)" };

  if (state === "idle" && !pinned) return null;

  const borderColor = !isActive
    ? "rgba(255, 255, 255, 0.1)"
    : state === "transcribing"
      ? transcribingColor.border
      : colors.border;

  const shadow = !isActive
    ? "none"
    : state === "recording"
      ? `0 0 12px ${colors.glow}`
      : state === "transcribing"
        ? `0 0 12px ${isCommand ? "rgba(168, 85, 247, 0.15)" : "rgba(59, 130, 246, 0.15)"}`
        : "none";

  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "transparent",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: isActive ? "8px" : "0px",
          background: isActive ? "rgba(20, 20, 28, 0.95)" : "rgba(20, 20, 28, 0.80)",
          borderRadius: "999px",
          // Idle: slim pill bar (50×8). Active: full pill.
          padding: isActive ? "7px 16px" : "0px",
          width: isActive ? "auto" : "50px",
          maxWidth: isActive ? "260px" : "50px",
          height: isActive ? "34px" : "8px",
          border: `1px solid ${borderColor}`,
          boxShadow: shadow,
          overflow: "hidden",
          transition:
            "width 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94), " +
            "max-width 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94), " +
            "height 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94), " +
            "padding 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94), " +
            "gap 0.25s ease, " +
            "border-color 0.25s ease, " +
            "box-shadow 0.25s ease, " +
            "background 0.25s ease",
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
                  background: `${colors.accent} 0.25)`,
                  animation: "ping 1.5s cubic-bezier(0,0,0.2,1) infinite",
                }}
              />
              <div style={{ width: "10px", height: "10px", borderRadius: "50%", background: colors.solid }} />
            </div>
            <Waveform level={level} barCount={9} color={`${colors.accent} 1)`} />
            <span style={{ color: styleName ? "#c084fc" : colors.text, fontSize: "10px", fontWeight: 600, letterSpacing: "1px", whiteSpace: "nowrap" }}>
              {styleName ? t("pill.recording.labelWithStyle", { styleName }) : label}
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
                border: `2px solid ${transcribingColor.border}`,
                borderTopColor: transcribingColor.top,
                animation: "spin 0.8s linear infinite",
                flexShrink: 0,
              }}
            />
            <span style={{ color: transcribingColor.text, fontSize: "10px", fontWeight: 500, whiteSpace: "nowrap" }}>
              {elapsed > 0 ? t("pill.transcribing.elapsed", { seconds: elapsed }) : t("pill.transcribing.processing")}
            </span>
          </>
        )}

        {/* DONE */}
        {state === "done" && (
          <>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke={colors.solid} strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
              <path d="M5 13l4 4L19 7" />
            </svg>
            <span style={{ color: colors.solid, fontSize: "10px", fontWeight: 500, whiteSpace: "nowrap" }}>{t("pill.done")}</span>
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

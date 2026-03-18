import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

export function WaveformDisplay() {
  const [level, setLevel] = useState(0);

  useEffect(() => {
    const unlisten = listen<number>("audio-level", (event) => {
      const normalized = Math.min(event.payload * 200, 100);
      setLevel(normalized);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const getBarColor = (level: number) => {
    if (level > 70) return "bg-red-500";
    if (level > 40) return "bg-yellow-500";
    return "bg-green-500";
  };

  return (
    <div className="mt-4 space-y-2">
      <div className="flex items-center gap-3">
        <span className="text-xs text-gray-400 w-8 text-right">
          {Math.round(level)}%
        </span>
        <div className="flex-1 h-3 bg-gray-800 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-75 ${getBarColor(level)}`}
            style={{ width: `${level}%` }}
          />
        </div>
      </div>
      <p className="text-xs text-gray-500">Speak into your microphone to test</p>
    </div>
  );
}

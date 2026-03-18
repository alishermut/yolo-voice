import { useState, useEffect, useRef } from "react";

interface EqualizerBarsProps {
  level: number; // 0-100
}

export function EqualizerBars({ level }: EqualizerBarsProps) {
  const [bars, setBars] = useState<number[]>([0, 0, 0, 0, 0]);
  const animRef = useRef<number>(0);

  useEffect(() => {
    const update = () => {
      setBars((prev) =>
        prev.map(() => {
          // Base height from level + random variation for organic feel
          const base = level * 0.8;
          const variation = (Math.random() - 0.5) * level * 0.4;
          return Math.max(8, Math.min(100, base + variation));
        }),
      );
      animRef.current = requestAnimationFrame(update);
    };

    // Run at ~15fps for a natural look
    const interval = setInterval(() => {
      cancelAnimationFrame(animRef.current);
      animRef.current = requestAnimationFrame(update);
    }, 66);

    return () => {
      clearInterval(interval);
      cancelAnimationFrame(animRef.current);
    };
  }, [level]);

  return (
    <div className="flex items-end gap-[2px] h-6">
      {bars.map((height, i) => (
        <div
          key={i}
          className="w-[3px] rounded-full bg-red-400 transition-all duration-75"
          style={{ height: `${height}%` }}
        />
      ))}
    </div>
  );
}

import { useState, useEffect } from "react";
import { WaveformDisplay } from "./WaveformDisplay";
import { Select } from "./ui/Select";
import type { DeviceInfo } from "../shared/types";
import { listDevices, startTest, stopTest } from "../shared/platform";

interface MicSelectorProps {
  deviceIndex?: number;
  onDeviceChange?: (index: number) => void;
}

export function MicSelector({ deviceIndex, onDeviceChange }: MicSelectorProps) {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selected, setSelected] = useState<number>(deviceIndex ?? 0);
  const [testing, setTesting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listDevices()
      .then((devs) => {
        setDevices(devs);
        if (deviceIndex !== undefined) {
          setSelected(deviceIndex);
        } else if (devs.length > 0) {
          setSelected(devs[0].index);
        }
      })
      .catch((e) => setError(String(e)));
  }, []);

  const handleTest = async () => {
    setError(null);
    try {
      if (testing) {
        await stopTest();
        setTesting(false);
      } else {
        await startTest(selected);
        setTesting(true);
      }
    } catch (e) {
      setError(String(e));
      setTesting(false);
    }
  };

  const handleDeviceChange = async (index: number) => {
    setSelected(index);
    onDeviceChange?.(index);
    if (testing) {
      try {
        await stopTest();
        setTesting(false);
      } catch (e) {
        setError(String(e));
      }
    }
  };

  if (devices.length === 0 && !error) {
    return (
      <p className="text-text-muted text-sm">No microphones found</p>
    );
  }

  const savedDevice = devices.find((d) => d.index === deviceIndex);

  return (
    <div>
      {error && (
        <div className="mb-3 px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-sm">
          {error}
        </div>
      )}

      {savedDevice && (
        <div className="mb-2 flex items-center gap-2 text-xs text-text-secondary">
          <span className="inline-block w-2 h-2 rounded-full bg-success" />
          Active: <span className="text-text-primary font-medium">{savedDevice.name}</span>
        </div>
      )}

      <div className="flex items-center gap-3">
        <Select
          value={String(selected)}
          onChange={(v) => handleDeviceChange(Number(v))}
          options={devices.map((d) => ({
            value: String(d.index),
            label: `${d.name}${d.index === deviceIndex ? " (active)" : ""}`,
          }))}
          className="flex-1"
        />

        <button
          onClick={handleTest}
          className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg-base ${
            testing
              ? "bg-error hover:bg-error text-white"
              : "bg-accent hover:bg-accent-hover text-white"
          }`}
        >
          {testing ? "Stop" : "Test"}
        </button>
      </div>

      {testing && <WaveformDisplay />}
    </div>
  );
}

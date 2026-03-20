import { useState, useEffect } from "react";
import { WaveformDisplay } from "./WaveformDisplay";
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
      <p className="text-gray-500 text-sm">No microphones found</p>
    );
  }

  const savedDevice = devices.find((d) => d.index === deviceIndex);

  return (
    <div>
      {error && (
        <div className="mb-3 px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
          {error}
        </div>
      )}

      {savedDevice && (
        <div className="mb-2 flex items-center gap-2 text-xs text-gray-400">
          <span className="inline-block w-2 h-2 rounded-full bg-green-500" />
          Active: <span className="text-gray-300 font-medium">{savedDevice.name}</span>
        </div>
      )}

      <div className="flex items-center gap-3">
        <select
          value={selected}
          onChange={(e) => handleDeviceChange(Number(e.target.value))}
          className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
        >
          {devices.map((d) => (
            <option key={d.index} value={d.index}>
              {d.name}{d.index === deviceIndex ? " (active)" : ""}
            </option>
          ))}
        </select>

        <button
          onClick={handleTest}
          className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
            testing
              ? "bg-red-600 hover:bg-red-700 text-white"
              : "bg-blue-600 hover:bg-blue-700 text-white"
          }`}
        >
          {testing ? "Stop" : "Test"}
        </button>
      </div>

      {testing && <WaveformDisplay />}
    </div>
  );
}

import { useState, useCallback } from "react";

type ToastType = "success" | "error" | "info";

interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
  visible: boolean;
}

let nextId = 0;

export function useToast() {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const addToast = useCallback((message: string, type: ToastType = "info") => {
    const id = nextId++;
    setToasts((prev) => [...prev, { id, message, type, visible: true }]);

    // Start fade-out after 2.5s
    setTimeout(() => {
      setToasts((prev) =>
        prev.map((t) => (t.id === id ? { ...t, visible: false } : t)),
      );
    }, 2500);

    // Remove from DOM after fade-out animation
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3000);
  }, []);

  return { toasts, addToast };
}

const typeStyles: Record<ToastType, string> = {
  success: "bg-success-muted/90 border-success text-success",
  error: "bg-error-muted/90 border-error text-error",
  info: "bg-accent-muted/90 border-accent text-accent",
};

const typeIcons: Record<ToastType, string> = {
  success: "\u2713",
  error: "\u2717",
  info: "\u2139",
};

interface ToastContainerProps {
  toasts: ToastItem[];
}

export function ToastContainer({ toasts }: ToastContainerProps) {
  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`px-4 py-2.5 rounded-lg border text-sm font-medium shadow-lg transition-all duration-300 ${
            typeStyles[toast.type]
          } ${toast.visible ? "opacity-100 translate-x-0" : "opacity-0 translate-x-4"}`}
        >
          <span className="mr-2">{typeIcons[toast.type]}</span>
          {toast.message}
        </div>
      ))}
    </div>
  );
}

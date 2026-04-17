import {
  badgeStyles,
  buttonVariants,
  focusRing,
} from "../ui/styles";

type TrustBadgeTone = "local" | "cloud" | "neutral";
type TrustMessageTone = "error" | "info" | "success";

interface TrustPath {
  label: string;
  value: string;
}

interface TrustAction {
  label: string;
  onClick: () => void | Promise<void>;
  disabled?: boolean;
}

interface TrustCardProps {
  title: string;
  description: string[];
  badgeLabel?: string;
  badgeTone?: TrustBadgeTone;
  paths?: TrustPath[];
  actions?: TrustAction[];
  message?: { tone: TrustMessageTone; text: string } | null;
}

function badgeToneClass(tone: TrustBadgeTone) {
  switch (tone) {
    case "local":
      return "bg-success/15 text-success";
    case "cloud":
      return "bg-warning/15 text-warning";
    default:
      return "bg-bg-hover text-text-secondary";
  }
}

function messageToneClass(tone: TrustMessageTone) {
  switch (tone) {
    case "error":
      return "border-error bg-error-muted text-error";
    case "success":
      return "border-success bg-success/10 text-success";
    default:
      return "border-border-default bg-bg-base text-text-secondary";
  }
}

export function TrustCard({
  title,
  description,
  badgeLabel,
  badgeTone = "neutral",
  paths = [],
  actions = [],
  message = null,
}: TrustCardProps) {
  return (
    <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-4">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <h3 className="text-sm font-semibold text-text-primary">{title}</h3>
          <div className="space-y-1">
            {description.map((line, index) => (
              <p key={`${title}-line-${index}`} className="text-xs text-text-muted">
                {line}
              </p>
            ))}
          </div>
        </div>
        {badgeLabel && (
          <span className={`${badgeStyles} ${badgeToneClass(badgeTone)} shrink-0`}>
            {badgeLabel}
          </span>
        )}
      </div>

      {paths.length > 0 && (
        <div className="space-y-2">
          {paths.map((path) => (
            <div key={path.label} className="space-y-1">
              <p className="text-xs font-medium text-text-primary">{path.label}</p>
              <p className="text-xs text-text-muted break-all font-mono">{path.value}</p>
            </div>
          ))}
        </div>
      )}

      {actions.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {actions.map((action) => (
            <button
              key={action.label}
              onClick={() => {
                void action.onClick();
              }}
              disabled={action.disabled}
              className={`${buttonVariants.secondary} text-xs ${focusRing}`}
            >
              {action.label}
            </button>
          ))}
        </div>
      )}

      {message && (
        <div className={`px-3 py-2 border rounded-lg text-xs break-all ${messageToneClass(message.tone)}`}>
          {message.text}
        </div>
      )}
    </div>
  );
}

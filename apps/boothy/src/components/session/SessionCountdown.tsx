import clsx from 'clsx';
import { Timer } from 'lucide-react';

interface SessionCountdownProps {
  remainingSeconds: number | null;
  size?: 'sm' | 'md' | 'lg';
}

function formatRemainingTime(remainingSeconds: number) {
  const clamped = Math.max(0, Math.floor(remainingSeconds));
  const minutes = Math.floor(clamped / 60);
  const seconds = clamped % 60;
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

export default function SessionCountdown({ remainingSeconds, size = 'md' }: SessionCountdownProps) {
  if (remainingSeconds === null || Number.isNaN(remainingSeconds)) {
    return null;
  }

  const display = formatRemainingTime(remainingSeconds);

  // Determine urgency level based on remaining time
  const isUrgent = remainingSeconds <= 60; // 1 minute or less
  const isWarning = remainingSeconds <= 300 && remainingSeconds > 60; // 5 minutes or less
  const isNormal = remainingSeconds > 300;

  // Size configurations
  const sizeConfig = {
    sm: {
      container: 'px-3 py-1.5 gap-1.5',
      icon: 14,
      text: 'text-sm',
    },
    md: {
      container: 'px-4 py-2 gap-2',
      icon: 18,
      text: 'text-lg',
    },
    lg: {
      container: 'px-5 py-2.5 gap-2.5',
      icon: 22,
      text: 'text-xl',
    },
  };

  const config = sizeConfig[size];

  return (
    <div
      className={clsx(
        'flex items-center rounded-full font-bold tracking-wide shadow-lg transition-all duration-300',
        'backdrop-blur-md border',
        config.container,
        config.text,
        {
          // Normal state (> 5 min): accent colored, calm
          'bg-accent/20 border-accent/30 text-accent': isNormal,
          // Warning state (1-5 min): orange/amber warning
          'bg-amber-500/20 border-amber-400/40 text-amber-400': isWarning,
          // Urgent state (< 1 min): red + pulsing animation
          'bg-red-500/25 border-red-400/50 text-red-400 animate-pulse': isUrgent,
        }
      )}
      aria-label={`Session time remaining ${display}`}
      role="timer"
    >
      <Timer
        size={config.icon}
        className={clsx(
          'transition-colors duration-300',
          {
            'text-accent': isNormal,
            'text-amber-400': isWarning,
            'text-red-400': isUrgent,
          }
        )}
      />
      <span className="tabular-nums font-mono">{display}</span>

      {/* Visual indicator dot */}
      <div
        className={clsx(
          'w-2 h-2 rounded-full transition-all duration-300',
          {
            'bg-accent': isNormal,
            'bg-amber-400 animate-pulse': isWarning,
            'bg-red-400 animate-ping': isUrgent,
          }
        )}
      />
    </div>
  );
}

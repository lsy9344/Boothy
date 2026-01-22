import { useCallback, useState, useEffect } from 'react';
import { platform } from '@tauri-apps/plugin-os';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X, Shield, User } from 'lucide-react';
import clsx from 'clsx';

interface TitleBarProps {
  boothyMode?: 'customer' | 'admin';
  boothyHasAdminPassword?: boolean;
  adminOverrideActive?: boolean;
  isAdminActionRunning?: boolean;
  onAdminToggle?: () => void;
}

export default function TitleBar({
  boothyMode = 'customer',
  boothyHasAdminPassword = false,
  adminOverrideActive = false,
  isAdminActionRunning = false,
  onAdminToggle,
}: TitleBarProps) {
  const isTauriRuntime = typeof window !== 'undefined' && '__TAURI__' in window;
  const [osPlatform, setOsPlatform] = useState(isTauriRuntime ? '' : 'web');

  useEffect(() => {
    if (!isTauriRuntime) {
      return;
    }
    const getPlatform = async () => {
      try {
        const p = platform();
        setOsPlatform(p);
      } catch (error) {
        console.error('Failed to get platform:', error);
        setOsPlatform('windows');
      }
    };
    getPlatform();
  }, [isTauriRuntime]);

  const appWindow = isTauriRuntime ? getCurrentWindow() : null;
  const handleMinimize = () => appWindow?.minimize();
  const handleClose = () => appWindow?.close();

  const handleMaximize = useCallback(async () => {
    if (!appWindow) {
      return;
    }
    switch (osPlatform) {
      case 'macos': {
        const isFullscreen = await appWindow.isFullscreen();
        appWindow.setFullscreen(!isFullscreen);
        break;
      }
      default:
        appWindow.toggleMaximize();
        break;
    }
  }, [osPlatform, appWindow]);

  const isMac = osPlatform === 'macos';
  const isWindows = osPlatform === 'windows';

  if (!osPlatform) {
    return <div className="h-10 fixed top-0 left-0 right-0 z-50" data-tauri-drag-region />;
  }

  const isAdmin = boothyMode === 'admin';

  return (
    <div
      className="h-10 bg-bg-secondary border-white/5 flex justify-between items-center select-none fixed top-0 left-0 right-0 z-50"
      data-tauri-drag-region
    >
      <div className="flex items-center h-full">
        {isMac && (
          <div className="flex items-center h-full px-4 space-x-2">
            <button
              aria-label="Close window"
              className="w-3 h-3 rounded-full bg-red-500 hover:bg-red-600 transition-colors duration-150"
              onClick={handleClose}
            />
            <button
              aria-label="Minimize window"
              className="w-3 h-3 rounded-full bg-yellow-500 hover:bg-yellow-600 transition-colors duration-150"
              onClick={handleMinimize}
            />
            <button
              aria-label="Maximize window"
              className="w-3 h-3 rounded-full bg-green-500 hover:bg-green-600 transition-colors duration-150"
              onClick={handleMaximize}
            />
          </div>
        )}
        <div data-tauri-drag-region className={`flex items-center h-full ${isMac ? '' : 'px-4'}`}>
          <p className="text-sm font-semibold text-text-secondary">RapidRAW</p>
        </div>
      </div>

      {/* Mode Toggle - Absolute Center */}
      {onAdminToggle && (
        <div className="absolute left-1/2 -translate-x-1/2 flex items-center">
          {/* Container with glass effect and border */}
          <div className="flex items-center bg-black/30 backdrop-blur-sm rounded-full border border-white/10 p-0.5">
            {/* Customer Button */}
            <button
              aria-label="Customer mode"
              className={clsx(
                'flex items-center justify-center gap-1.5 w-28 px-3 py-1.5 rounded-full text-xs font-medium transition-all duration-200',
                !isAdmin
                  ? 'bg-accent text-button-text shadow-sm'
                  : 'text-text-secondary hover:text-text-primary hover:bg-white/5',
              )}
              onClick={(e) => {
                e.stopPropagation();
                if (isAdmin && !isAdminActionRunning) {
                  onAdminToggle();
                }
              }}
              disabled={!isAdmin || isAdminActionRunning}
            >
              <User size={14} />
              <span>Customer</span>
            </button>
            {/* Admin Button */}
            <button
              aria-label="Toggle admin mode"
              className={clsx(
                'flex items-center justify-center gap-1.5 w-28 px-3 py-1.5 rounded-full text-xs font-medium transition-all duration-200',
                isAdmin
                  ? 'bg-accent text-button-text shadow-sm'
                  : 'text-text-secondary hover:text-text-primary hover:bg-white/5',
                isAdminActionRunning && 'opacity-60 cursor-not-allowed',
              )}
              disabled={isAdminActionRunning}
              onClick={(e) => {
                e.stopPropagation();
                if (!isAdmin) {
                  onAdminToggle();
                }
              }}
            >
              <Shield size={14} />
              <span>Admin</span>
            </button>
          </div>
          {/* Override Active Badge */}
          {isAdmin && adminOverrideActive && (
            <span className="ml-2 px-2.5 py-1 rounded-full text-xs font-semibold text-amber-300 bg-amber-500/20 backdrop-blur-sm border border-amber-400/40">
              Override Active
            </span>
          )}
        </div>
      )}

      <div className="flex items-center h-full">
        {isWindows && (
          <>
            <button
              aria-label="Minimize window"
              className="p-2 h-full inline-flex justify-center items-center hover:bg-white/10 transition-colors duration-150"
              onClick={handleMinimize}
            >
              <Minus size={16} className="text-text-secondary" />
            </button>
            <button
              aria-label="Maximize window"
              className="p-2 h-full inline-flex justify-center items-center hover:bg-white/10 transition-colors duration-150"
              onClick={handleMaximize}
            >
              <Square size={14} className="text-text-secondary" />
            </button>
            <button
              aria-label="Close window"
              className="p-2 h-full inline-flex justify-center items-center hover:bg-red-500/80 transition-colors duration-150"
              onClick={handleClose}
            >
              <X size={16} className="text-text-secondary hover:text-white" />
            </button>
          </>
        )}
      </div>
    </div>
  );
}

import { useState } from 'react';
import { Camera, ChevronDown, ChevronUp } from 'lucide-react';

const CAMERA_MODES = ['Manual', 'Aperture Priority', 'Shutter Priority', 'Program Auto'];
const ISO_OPTIONS = ['Auto', '100', '200', '400', '800', '1600', '3200'];
const SHUTTER_OPTIONS = ['1/30', '1/60', '1/125', '1/250', '1/500'];
const APERTURE_OPTIONS = ['f/1.8', 'f/2.8', 'f/4', 'f/5.6', 'f/8'];
const WHITE_BALANCE_OPTIONS = ['Auto', 'Daylight', 'Cloudy', 'Tungsten', 'Fluorescent'];

interface CameraControlsPanelProps {
  title?: string;
}

export default function CameraControlsPanel({ title = 'Camera Controls' }: CameraControlsPanelProps) {
  const [isSettingsOpen, setIsSettingsOpen] = useState(true);
  const isConnected = false;

  const selectClassName =
    'w-full bg-bg-primary border border-border-color rounded-md px-2 py-1.5 text-sm text-text-primary disabled:opacity-50 disabled:cursor-not-allowed';

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 flex justify-between items-center flex-shrink-0 border-b border-surface">
        <div className="flex items-center gap-2">
          <h2 className="text-xl font-bold text-primary text-shadow-shiny">{title}</h2>
          <span className="text-xs bg-accent/20 text-accent px-2 py-0.5 rounded-full">Admin</span>
        </div>
      </div>

      <div className="flex-grow overflow-y-auto p-4">
        <div className="flex items-center justify-between gap-2 mb-4">
          <div className="flex items-center gap-2">
            <span className={`h-2.5 w-2.5 rounded-full ${isConnected ? 'bg-green-400' : 'bg-red-500'}`} />
            <span className="text-sm text-text-secondary">
              {isConnected ? 'Camera connected' : 'Camera not connected'}
            </span>
          </div>
        </div>

        <div className="bg-surface rounded-lg border border-border-color">
          <button
            className="w-full p-3 flex items-center justify-between text-left hover:bg-bg-primary/50 transition-colors rounded-t-lg"
            onClick={() => setIsSettingsOpen((prev) => !prev)}
            type="button"
          >
            <div className="flex items-center gap-2">
              <Camera size={16} className="text-text-secondary" />
              <span className="text-sm font-medium text-text-primary">Camera Settings</span>
            </div>
            {isSettingsOpen ? (
              <ChevronUp size={16} className="text-text-secondary" />
            ) : (
              <ChevronDown size={16} className="text-text-secondary" />
            )}
          </button>

          {isSettingsOpen && (
            <div className="p-3 pt-0 space-y-3">
              <label className="flex flex-col gap-1 text-xs text-text-secondary">
                Mode
                <select className={selectClassName} defaultValue={CAMERA_MODES[0]} disabled={!isConnected}>
                  {CAMERA_MODES.map((mode) => (
                    <option key={mode} value={mode}>
                      {mode}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1 text-xs text-text-secondary">
                ISO
                <select className={selectClassName} defaultValue={ISO_OPTIONS[1]} disabled={!isConnected}>
                  {ISO_OPTIONS.map((iso) => (
                    <option key={iso} value={iso}>
                      {iso}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1 text-xs text-text-secondary">
                Shutter Speed
                <select className={selectClassName} defaultValue={SHUTTER_OPTIONS[2]} disabled={!isConnected}>
                  {SHUTTER_OPTIONS.map((shutter) => (
                    <option key={shutter} value={shutter}>
                      {shutter}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1 text-xs text-text-secondary">
                Aperture
                <select className={selectClassName} defaultValue={APERTURE_OPTIONS[1]} disabled={!isConnected}>
                  {APERTURE_OPTIONS.map((aperture) => (
                    <option key={aperture} value={aperture}>
                      {aperture}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1 text-xs text-text-secondary">
                White Balance
                <select className={selectClassName} defaultValue={WHITE_BALANCE_OPTIONS[0]} disabled={!isConnected}>
                  {WHITE_BALANCE_OPTIONS.map((balance) => (
                    <option key={balance} value={balance}>
                      {balance}
                    </option>
                  ))}
                </select>
              </label>
              <p className="text-xs text-text-secondary pt-2 border-t border-border-color">
                Camera controls will activate once the sidecar reports a connected camera.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

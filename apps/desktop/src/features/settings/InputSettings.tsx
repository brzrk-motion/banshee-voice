type AudioInputDevice = {
  id: string;
  name: string;
  isDefault: boolean;
  channels?: number | null;
  sampleRateHz?: number | null;
};

type Settings = {
  microphoneDeviceId?: string | null;
  vadSensitivity: number;
  pushToTalkShortcut: string;
  toggleRecordingShortcut: string;
  cancelShortcut: string;
  autoPasteEnabled: boolean;
  preserveClipboard: boolean;
};

type Props = {
  devices: AudioInputDevice[];
  settings: Settings | null;
  onUpdate: (payload: Record<string, unknown>) => Promise<void>;
};

export function InputSettings({ devices, settings, onUpdate }: Props) {
  if (!settings) {
    return <section className="card muted">Loading local input settings...</section>;
  }

  return (
    <section className="card stack-gap">
      <div>
        <div className="section-label">Input Settings</div>
        <h2>Microphone, shortcuts, and fallback behavior</h2>
      </div>

      <label className="field">
        <span>Microphone</span>
        <select
          value={settings.microphoneDeviceId ?? ""}
          onChange={(event) =>
            void onUpdate({ microphoneDeviceId: event.target.value || null })
          }
        >
          <option value="">System default</option>
          {devices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name}
              {device.isDefault ? " (default)" : ""}
            </option>
          ))}
        </select>
      </label>

      <label className="field">
        <span>VAD Sensitivity</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={settings.vadSensitivity}
          onChange={(event) =>
            void onUpdate({ vadSensitivity: Number(event.target.value) })
          }
        />
      </label>

      <div className="grid-two">
        <label className="field">
          <span>Speech Model</span>
          <select value="whisper-preview" disabled>
            <option value="whisper-preview">Whisper Preview Adapter</option>
          </select>
        </label>
        <label className="field">
          <span>Push To Talk</span>
          <input
            value={settings.pushToTalkShortcut}
            onChange={(event) =>
              void onUpdate({ pushToTalkShortcut: event.target.value })
            }
          />
        </label>
        <label className="field">
          <span>Toggle Recording</span>
          <input
            value={settings.toggleRecordingShortcut}
            onChange={(event) =>
              void onUpdate({ toggleRecordingShortcut: event.target.value })
            }
          />
        </label>
        <label className="field">
          <span>Cancel</span>
          <input
            value={settings.cancelShortcut}
            onChange={(event) => void onUpdate({ cancelShortcut: event.target.value })}
          />
        </label>
      </div>

      <div className="toggles">
        <label>
          <input
            type="checkbox"
            checked={settings.autoPasteEnabled}
            onChange={(event) =>
              void onUpdate({ autoPasteEnabled: event.target.checked })
            }
          />
          Auto-paste when the session permits it
        </label>
        <label>
          <input
            type="checkbox"
            checked={settings.preserveClipboard}
            onChange={(event) =>
              void onUpdate({ preserveClipboard: event.target.checked })
            }
          />
          Preserve the prior clipboard when possible
        </label>
      </div>
    </section>
  );
}

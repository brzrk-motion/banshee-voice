type Dashboard = {
  privacyMode: string;
  transcriptionsToday: number;
  wordsToday: number;
  speechMinutesToday: number;
  microphoneName?: string | null;
  speechModelName?: string | null;
  activeProfileName?: string | null;
  pushToTalkShortcut: string;
  sessionType: string;
};

type RecordingResult = {
  sessionId: string;
  rawText: string;
  deterministicText: string;
  finalText: string;
  sttBackend: string;
  peakLevel: number;
  status: string;
  outputMethod: string;
  outputResult: string;
  outputMessage: string;
  applicationName: string;
  windowTitle: string;
  durationMs: number;
};

type Props = {
  dashboard: Dashboard | null;
  recordingState: string;
  latestResult: RecordingResult | null;
  onStart: () => Promise<void>;
  onStop: () => Promise<void>;
  onCancel: () => Promise<void>;
};

export function DashboardPage({
  dashboard,
  recordingState,
  latestResult,
  onStart,
  onStop,
  onCancel,
}: Props) {
  return (
    <section className="card stack-gap">
      <div className="hero-row">
        <div>
          <div className="section-label">Dashboard</div>
          <h1>Fast push-to-talk dictation</h1>
          <p className="muted">
            This preview slice exercises the local recording pipeline, VAD cleanup,
            HUD updates, and clipboard-first output flow.
          </p>
        </div>
        <div className={`status-pill status-pill--${recordingState}`}>
          {recordingState}
        </div>
      </div>

      <div className="metrics-grid">
        <article className="metric-card">
          <span>Privacy</span>
          <strong>{dashboard?.privacyMode ?? "local_only"}</strong>
        </article>
        <article className="metric-card">
          <span>Microphone</span>
          <strong>{dashboard?.microphoneName ?? "System default"}</strong>
        </article>
        <article className="metric-card">
          <span>Speech Model</span>
          <strong>{dashboard?.speechModelName ?? "Whisper Preview"}</strong>
        </article>
        <article className="metric-card">
          <span>Shortcut</span>
          <strong>{dashboard?.pushToTalkShortcut ?? "Ctrl+Shift+Space"}</strong>
        </article>
      </div>

      <div className="control-row">
        <button className="primary" onClick={() => void onStart()}>
          Start Recording
        </button>
        <button onClick={() => void onStop()}>Stop And Transcribe</button>
        <button onClick={() => void onCancel()}>Cancel</button>
      </div>

      <div className="card inset stack-gap">
        <div className="section-label">Latest Run</div>
        {latestResult ? (
          <>
            <div className="result-grid">
              <div>
                <span className="meta-label">Destination</span>
                <strong>
                  {latestResult.applicationName} / {latestResult.windowTitle}
                </strong>
              </div>
              <div>
                <span className="meta-label">Output</span>
                <strong>
                  {latestResult.outputMethod} / {latestResult.outputResult} / {latestResult.status}
                </strong>
              </div>
            </div>
            <div className="result-grid">
              <div>
                <span className="meta-label">Session</span>
                <strong>{latestResult.sessionId}</strong>
              </div>
              <div>
                <span className="meta-label">Backend</span>
                <strong>{latestResult.sttBackend}</strong>
              </div>
              <div>
                <span className="meta-label">Peak Level</span>
                <strong>{latestResult.peakLevel.toFixed(2)}</strong>
              </div>
            </div>
            <div>
              <span className="meta-label">Raw Transcript</span>
              <p className="transcript-block">{latestResult.rawText}</p>
            </div>
            <div>
              <span className="meta-label">Deterministic Cleanup</span>
              <p className="transcript-block">{latestResult.deterministicText}</p>
            </div>
            <div>
              <span className="meta-label">Final Transcript</span>
              <p className="transcript-block">{latestResult.finalText}</p>
            </div>
            <p className="muted">{latestResult.outputMessage}</p>
          </>
        ) : (
          <p className="muted">No recording has completed yet.</p>
        )}
      </div>
    </section>
  );
}

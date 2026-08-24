import { LoaderCircle, Save } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import type { AudioInputDevice, DictionaryEntry, ModelStatus, Settings } from "@/lib/types";
import { ShortcutCapture } from "./ShortcutCapture";

type Props = {
  settings: Settings | null;
  devices: AudioInputDevice[];
  vocabulary: DictionaryEntry[];
  cleanupStatus: ModelStatus;
  saving: boolean;
  onSave: (settings: Settings, vocabulary: DictionaryEntry[]) => Promise<void>;
  onRetryCleanup: () => Promise<void>;
};

function formatVocabulary(entries: DictionaryEntry[]) {
  return entries.map((entry) => entry.spokenForm === entry.outputForm ? entry.outputForm : `${entry.spokenForm} => ${entry.outputForm}`).join("\n");
}

function parseVocabulary(value: string): DictionaryEntry[] {
  return value.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).map((line) => {
    const separator = line.indexOf("=>");
    if (separator < 0) return { spokenForm: line, outputForm: line };
    return { spokenForm: line.slice(0, separator).trim(), outputForm: line.slice(separator + 2).trim() };
  });
}

function SettingRow({ title, description, children }: { title: string; description: string; children: React.ReactNode }) {
  return <div className="grid gap-3 border-t py-4 first:border-t-0 first:pt-0 sm:grid-cols-[minmax(0,1fr)_260px] sm:items-center"><div><Label>{title}</Label><p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p></div><div>{children}</div></div>;
}

export function SettingsPage({ settings, devices, vocabulary, cleanupStatus, saving, onSave, onRetryCleanup }: Props) {
  const [draft, setDraft] = useState<Settings | null>(settings);
  const [vocabularyDraft, setVocabularyDraft] = useState(formatVocabulary(vocabulary));
  useEffect(() => setDraft(settings), [settings]);
  useEffect(() => setVocabularyDraft(formatVocabulary(vocabulary)), [vocabulary]);
  const dirty = useMemo(() => Boolean(draft && settings && (JSON.stringify(draft) !== JSON.stringify(settings) || vocabularyDraft !== formatVocabulary(vocabulary))), [draft, settings, vocabulary, vocabularyDraft]);
  const update = <K extends keyof Settings>(key: K, value: Settings[K]) => setDraft((current) => current ? { ...current, [key]: value } : current);

  if (!draft) return <div className="mx-auto max-w-4xl space-y-4 p-6 lg:p-8"><div className="h-48 animate-pulse rounded-xl bg-muted" /><div className="h-48 animate-pulse rounded-xl bg-muted" /></div>;

  return (
    <div className="mx-auto max-w-4xl space-y-5 p-6 pb-28 lg:p-8 lg:pb-28">
      <Card>
        <CardHeader><CardTitle>Microphone</CardTitle><CardDescription>Choose the input Banshee listens to and tune speech detection.</CardDescription></CardHeader>
        <CardContent>
          <SettingRow title="Input device" description="Use the system default or choose a specific microphone.">
            <NativeSelect aria-label="Input device" value={draft.microphoneDeviceId ?? ""} onChange={(event) => update("microphoneDeviceId", event.target.value || null)}>
              <option value="">System default</option>
              {devices.map((device) => <option key={device.id} value={device.id}>{device.name}{device.isDefault ? " (default)" : ""}</option>)}
            </NativeSelect>
          </SettingRow>
          <SettingRow title="Voice sensitivity" description="Higher values require a stronger voice signal.">
            <div className="flex items-center gap-4"><Slider aria-label="Voice sensitivity" min={0} max={1} step={0.05} value={[draft.vadSensitivity]} onValueChange={([value]) => update("vadSensitivity", value)} /><span className="w-10 text-right text-xs tabular-nums text-muted-foreground">{Math.round(draft.vadSensitivity * 100)}%</span></div>
          </SettingRow>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>HUD</CardTitle><CardDescription>Control the compact recording overlay and its global shortcut.</CardDescription></CardHeader>
        <CardContent>
          <SettingRow title="Push-to-talk shortcut" description="Focus the field, then press the complete shortcut you want to use."><ShortcutCapture value={draft.pushToTalkShortcut} onChange={(value) => update("pushToTalkShortcut", value)} /></SettingRow>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>Output</CardTitle><CardDescription>Choose how HUD recordings hand text back to other applications.</CardDescription></CardHeader>
        <CardContent>
          <SettingRow title="Preserve clipboard" description="Restore the previous clipboard when the output backend can do so."><Switch aria-label="Preserve clipboard" checked={draft.preserveClipboard} onCheckedChange={(value) => update("preserveClipboard", value)} /></SettingRow>
          <SettingRow title="Clipboard restore delay" description="Keep the transcript available briefly while the target consumes the paste."><div className="relative"><Input aria-label="Clipboard restore delay" type="number" min={0} max={2000} value={draft.pasteDelayMs} onChange={(event) => update("pasteDelayMs", Number(event.target.value))} /><span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">ms</span></div></SettingRow>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>Application</CardTitle><CardDescription>Control startup, tray, and feedback preferences.</CardDescription></CardHeader>
        <CardContent>
          <SettingRow title="Launch at login" description="Start Banshee when you sign in."><Switch aria-label="Launch at login" checked={draft.launchAtLogin} onCheckedChange={(value) => update("launchAtLogin", value)} /></SettingRow>
          <SettingRow title="Start minimized" description="Open directly into the background."><Switch aria-label="Start minimized" checked={draft.startMinimized} onCheckedChange={(value) => update("startMinimized", value)} /></SettingRow>
          <SettingRow title="Start sound" description="Play a cue when recording begins."><Switch aria-label="Start sound" checked={draft.playStartSound} onCheckedChange={(value) => update("playStartSound", value)} /></SettingRow>
          <SettingRow title="Completion sound" description="Play a cue when transcription completes."><Switch aria-label="Completion sound" checked={draft.playCompletionSound} onCheckedChange={(value) => update("playCompletionSound", value)} /></SettingRow>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>Processing & privacy</CardTitle><CardDescription>Transcription stays local. History contains text only.</CardDescription></CardHeader>
        <CardContent>
          <SettingRow title="Acceleration" description="This build currently runs local inference on CPU."><NativeSelect aria-label="Acceleration" value={draft.accelerationPreference} onChange={(event) => update("accelerationPreference", event.target.value as Settings["accelerationPreference"])}><option value="auto">Automatic (CPU)</option><option value="cpu">CPU</option><option value="gpu" disabled>GPU unavailable in this build</option></NativeSelect></SettingRow>
          <SettingRow title="Cleanup model" description="Optionally refine transcripts locally. Missing or slow cleanup always falls back to deterministic text.">
            <div className="space-y-2">
              <div className="flex items-center justify-between gap-3"><Switch aria-label="Cleanup model" checked={draft.cleanupLlmEnabled} onCheckedChange={(value) => update("cleanupLlmEnabled", value)} /><span className="text-xs text-muted-foreground">{cleanupStatus.state === "downloading" && cleanupStatus.totalBytes ? `Downloading ${Math.round(cleanupStatus.downloadedBytes / cleanupStatus.totalBytes * 100)}%` : cleanupStatus.state === "ready" ? "Installed" : cleanupStatus.state === "error" ? "Download failed" : "Not installed"}</span></div>
              {cleanupStatus.state === "error" ? <Button type="button" size="sm" variant="outline" onClick={() => void onRetryCleanup()}>Retry cleanup download</Button> : null}
            </div>
          </SettingRow>
          <SettingRow title="Custom vocabulary" description="One canonical term per line, or use “heard phrase => Correct Term” for explicit aliases."><Textarea aria-label="Custom vocabulary" rows={7} value={vocabularyDraft} placeholder={"Banshee\nHUD\nbanci hud => Banshee HUD"} onChange={(event) => setVocabularyDraft(event.target.value)} /></SettingRow>
          <SettingRow title="Save text history" description="Store completed transcript text locally. Audio is never retained."><Switch aria-label="Save text history" checked={draft.historyEnabled} onCheckedChange={(value) => update("historyEnabled", value)} /></SettingRow>
        </CardContent>
      </Card>

      <div className="sticky bottom-0 z-10 flex justify-end border-t bg-background/95 px-6 py-4 backdrop-blur">
        <Button disabled={!dirty || saving} onClick={() => void onSave(draft, parseVocabulary(vocabularyDraft))}>{saving ? <LoaderCircle className="animate-spin" /> : <Save />}{saving ? "Saving..." : "Save changes"}</Button>
      </div>
    </div>
  );
}

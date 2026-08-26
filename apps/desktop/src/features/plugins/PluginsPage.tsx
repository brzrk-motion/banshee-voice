import { Box, Download, LoaderCircle, RotateCcw, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import type { PluginSummary } from "@/lib/types";
import { PluginSettingsDialog } from "./PluginSettingsDialog";

type Props = {
  plugins: PluginSummary[];
  changing: string | null;
  savingSettings: string | null;
  onToggle: (pluginId: string, enabled: boolean) => Promise<void>;
  onRetry: (pluginId: string) => Promise<void>;
  onSaveSettings: (pluginId: string, settings: Record<string, string>) => Promise<void>;
};

function statusText(plugin: PluginSummary) {
  if (!plugin.enabled) return "Off";
  if (plugin.runtimeState === "downloading" && plugin.totalBytes) {
    return `Downloading ${Math.round(plugin.downloadedBytes / plugin.totalBytes * 100)}%`;
  }
  return ({ missing: "Preparing model", downloading: "Downloading", loading: "Loading", ready: "Ready", error: "Setup failed" })[plugin.runtimeState];
}

export function PluginsPage({ plugins, changing, savingSettings, onToggle, onRetry, onSaveSettings }: Props) {
  return (
    <div className="mx-auto max-w-4xl space-y-5 p-6 lg:p-8">
      <div className="overflow-hidden rounded-2xl border bg-[radial-gradient(circle_at_top_right,var(--accent)_0,transparent_48%)] p-6">
        <div className="mb-4 grid size-11 place-items-center rounded-xl bg-foreground text-background"><Sparkles className="size-5" /></div>
        <h2 className="text-xl font-semibold tracking-tight">Shape your words after transcription</h2>
        <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">Plugins run locally in a fixed sequence after transcription. Turn Transcript Cleanup off to pass the raw transcript directly to later plugins and output.</p>
      </div>
      {plugins.length === 0 ? <Card><CardContent className="flex items-center gap-3 py-8 text-sm text-muted-foreground"><Box />No plugins are registered in this build.</CardContent></Card> : null}
      {plugins.map((plugin) => (
        <Card key={plugin.manifest.id}>
          <CardHeader className="flex-row items-start justify-between gap-4">
            <div><CardTitle>{plugin.manifest.name}</CardTitle><CardDescription className="mt-1 max-w-xl">{plugin.manifest.description}</CardDescription></div>
            <Switch aria-label={`Enable ${plugin.manifest.name}`} checked={plugin.enabled} disabled={changing === plugin.manifest.id} onCheckedChange={(enabled) => void onToggle(plugin.manifest.id, enabled)} />
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap items-center justify-between gap-3 border-t pt-4 text-xs text-muted-foreground">
              <div className="flex flex-wrap gap-x-4 gap-y-1"><span>{plugin.manifest.stage}</span><span>{plugin.manifest.author} · v{plugin.manifest.version}</span></div>
              <div className="flex items-center gap-2">
                {plugin.manifest.settings.length > 0 ? <PluginSettingsDialog plugin={plugin} saving={savingSettings === plugin.manifest.id} onSave={onSaveSettings} /> : null}
                {plugin.enabled && plugin.runtimeState !== "ready" && plugin.runtimeState !== "error" ? <LoaderCircle className="size-3.5 animate-spin" /> : null}
                {plugin.enabled && plugin.runtimeState === "downloading" ? <Download className="size-3.5" /> : null}
                <span>{statusText(plugin)}</span>
                {plugin.enabled && plugin.runtimeState === "error" ? <Button size="sm" variant="outline" onClick={() => void onRetry(plugin.manifest.id)}><RotateCcw />Retry</Button> : null}
              </div>
            </div>
            {plugin.message ? <p className="mt-3 text-xs text-destructive">{plugin.message}</p> : null}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

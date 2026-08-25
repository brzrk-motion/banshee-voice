import { useState } from "react";
import { Settings } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import type { PluginSummary } from "@/lib/types";

type Props = {
  plugin: PluginSummary;
  saving: boolean;
  onSave: (pluginId: string, settings: Record<string, string>) => Promise<void>;
};

export function PluginSettingsDialog({ plugin, saving, onSave }: Props) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<Record<string, string>>(plugin.settings);

  function changeOpen(next: boolean) {
    if (saving) return;
    if (next) setDraft(plugin.settings);
    setOpen(next);
  }

  async function save() {
    try {
      await onSave(plugin.manifest.id, draft);
      setOpen(false);
    } catch {
      // The application reports persistence failures through its shared toast.
    }
  }

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant="outline"><Settings />Settings</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{plugin.manifest.name} settings</DialogTitle>
          <DialogDescription>Configure how this plugin transforms finished transcripts.</DialogDescription>
        </DialogHeader>
        <div className="mt-6 space-y-5">
          {plugin.manifest.settings.map((setting) => (
            <div key={setting.key} className="space-y-2">
              <Label htmlFor={`${plugin.manifest.id}-${setting.key}`}>{setting.label}</Label>
              {setting.description ? <p className="text-xs leading-5 text-muted-foreground">{setting.description}</p> : null}
              <NativeSelect
                id={`${plugin.manifest.id}-${setting.key}`}
                value={draft[setting.key] ?? setting.defaultValue}
                disabled={saving}
                onChange={(event) => setDraft((current) => ({ ...current, [setting.key]: event.target.value }))}
              >
                {setting.options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </NativeSelect>
            </div>
          ))}
        </div>
        <DialogFooter>
          <DialogClose asChild><Button type="button" variant="outline" disabled={saving}>Cancel</Button></DialogClose>
          <Button type="button" disabled={saving} onClick={() => void save()}>{saving ? "Saving…" : "Save settings"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

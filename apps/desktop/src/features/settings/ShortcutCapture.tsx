import { Keyboard } from "lucide-react";
import { useState } from "react";
import { Input } from "@/components/ui/input";
import { shortcutFromKeyboardEvent } from "./shortcut";

export function ShortcutCapture({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const [capturing, setCapturing] = useState(false);
  return (
    <div className="relative">
      <Keyboard className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        className="pl-9"
        readOnly
        aria-label="Push-to-talk shortcut"
        value={capturing ? "Press shortcut…" : value}
        onFocus={() => setCapturing(true)}
        onBlur={() => setCapturing(false)}
        onKeyDown={(event) => {
          event.preventDefault();
          event.stopPropagation();
          const shortcut = shortcutFromKeyboardEvent(event.nativeEvent);
          if (shortcut) {
            onChange(shortcut);
            setCapturing(false);
            event.currentTarget.blur();
          }
        }}
      />
    </div>
  );
}

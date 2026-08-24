import { Check, Clipboard, LoaderCircle, Mic, Square, X } from "lucide-react";
import { useEffect, useRef } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import type { ModelStatus } from "@/lib/types";

type Props = {
  text: string;
  onTextChange: (text: string) => void;
  recordingState: string;
  completedSessionId: string | null;
  modelStatus: ModelStatus;
  onStart: () => Promise<void>;
  onStop: () => Promise<void>;
  onCancel: () => Promise<void>;
  onCopy: (text: string) => Promise<void>;
  onRetryModel: () => Promise<void>;
};

const busyStates = new Set(["stopping", "transcribing", "inserting"]);

export function TranscribePage({ text, onTextChange, recordingState, completedSessionId, modelStatus, onStart, onStop, onCancel, onCopy, onRetryModel }: Props) {
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const isRecording = recordingState === "recording";
  const isBusy = busyStates.has(recordingState);
  const modelReady = modelStatus.state === "ready";
  const downloadPercent = modelStatus.totalBytes
    ? Math.min(100, Math.round((modelStatus.downloadedBytes / modelStatus.totalBytes) * 100))
    : null;

  useEffect(() => {
    if (completedSessionId) editorRef.current?.focus();
  }, [completedSessionId]);

  return (
    <div className="mx-auto flex min-h-full max-w-5xl flex-col gap-5 p-6 lg:p-8">
      <Card className="flex min-h-[560px] flex-1 flex-col overflow-hidden">
        <CardHeader className="flex-row items-start justify-between gap-4 border-b">
          <div>
            <CardTitle>Scratch space</CardTitle>
            <CardDescription className="mt-1.5">Your latest transcription appears here. Edit freely before copying.</CardDescription>
          </div>
          <Badge variant={recordingState === "error" ? "destructive" : "outline"} className="gap-1.5 capitalize">
            {isRecording ? <span className="size-1.5 animate-pulse rounded-full bg-red-500" /> : null}
            {isBusy ? "Processing" : recordingState}
          </Badge>
        </CardHeader>
        <CardContent className="flex min-h-0 flex-1 flex-col gap-4 p-5">
          {!modelReady ? (
            <div className="flex items-center justify-between gap-4 rounded-lg border bg-muted/40 px-4 py-3 text-sm">
              <div>
                <div className="font-medium">
                  {modelStatus.state === "error" ? "Speech model download failed" : "Preparing local speech model"}
                </div>
                <div className="mt-0.5 text-muted-foreground">
                  {modelStatus.state === "downloading" && downloadPercent !== null
                    ? `Downloading ${modelStatus.modelName}: ${downloadPercent}%`
                    : modelStatus.message ?? (modelStatus.state === "loading" ? "Loading model into memory…" : "Starting download…")}
                </div>
              </div>
              {modelStatus.state === "error" ? <Button variant="outline" onClick={() => void onRetryModel()}>Retry</Button> : <LoaderCircle className="animate-spin" />}
            </div>
          ) : null}
          <Textarea
            ref={editorRef}
            aria-label="Transcription scratch space"
            className="min-h-[360px] flex-1 resize-none border-0 bg-transparent p-3 text-[15px] leading-7 shadow-none focus-visible:ring-0"
            placeholder="Press Start recording and speak. Your transcript will appear here…"
            value={text}
            onChange={(event) => onTextChange(event.target.value)}
          />
          <div className="flex flex-wrap items-center justify-between gap-3 border-t pt-4">
            <div className="flex items-center gap-2">
              {!isRecording ? (
                <Button size="lg" disabled={isBusy || !modelReady} onClick={() => void onStart()}>
                  {isBusy ? <LoaderCircle className="animate-spin" /> : <Mic />}
                  {isBusy ? "Transcribing…" : "Start recording"}
                </Button>
              ) : (
                <>
                  <Button size="lg" variant="destructive" onClick={() => void onStop()}>
                    <Square className="fill-current" /> Stop and transcribe
                  </Button>
                  <Button size="lg" variant="outline" onClick={() => void onCancel()}>
                    <X /> Cancel
                  </Button>
                </>
              )}
            </div>
            <Button variant="outline" disabled={!text.trim() || isBusy} onClick={() => void onCopy(text)}>
              {completedSessionId ? <Check /> : <Clipboard />}
              Copy text
            </Button>
          </div>
        </CardContent>
      </Card>
      <p className="text-center text-xs text-muted-foreground">Main-window recordings stay in this scratch space and are never pasted into another app.</p>
    </div>
  );
}

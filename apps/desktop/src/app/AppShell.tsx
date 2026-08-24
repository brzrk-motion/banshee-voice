import { History, Mic2, Settings as SettingsIcon, Waves } from "lucide-react";
import type { ReactNode } from "react";
import { Sidebar, SidebarContent, SidebarFooter, SidebarHeader, SidebarInset, SidebarMenuButton, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import type { PageId } from "@/lib/types";

const pageTitles: Record<PageId, { title: string; description: string }> = {
  transcribe: { title: "Transcribe", description: "Record an idea and shape it in your scratch space." },
  history: { title: "History", description: "Your locally stored transcripts, newest first." },
  settings: { title: "Settings", description: "Configure Banshee for your microphone and workflow." },
};

export function AppShell({ page, onNavigate, children }: { page: PageId; onNavigate: (page: PageId) => void; children: ReactNode }) {
  const heading = pageTitles[page];
  return (
    <SidebarProvider>
      <div className="flex h-screen w-full">
        <Sidebar>
          <SidebarHeader className="gap-3">
            <div className="grid size-9 shrink-0 place-items-center rounded-lg bg-foreground text-background">
              <Waves className="size-5" />
            </div>
            <div className="min-w-0 group-data-[collapsed=true]/sidebar:hidden">
              <div className="truncate text-sm font-semibold">Banshee</div>
              <div className="truncate text-xs text-muted-foreground">Voice workspace</div>
            </div>
          </SidebarHeader>
          <SidebarContent>
            <SidebarMenuButton label="Transcribe" active={page === "transcribe"} onClick={() => onNavigate("transcribe")}>
              <Mic2 />
            </SidebarMenuButton>
            <SidebarMenuButton label="History" active={page === "history"} onClick={() => onNavigate("history")}>
              <History />
            </SidebarMenuButton>
          </SidebarContent>
          <SidebarFooter>
            <SidebarMenuButton label="Settings" active={page === "settings"} onClick={() => onNavigate("settings")}>
              <SettingsIcon />
            </SidebarMenuButton>
          </SidebarFooter>
        </Sidebar>
        <SidebarInset>
          <header className="flex h-16 items-center gap-3 border-b px-5">
            <SidebarTrigger className="-ml-2" />
            <div className="min-w-0">
              <h1 className="truncate text-sm font-semibold">{heading.title}</h1>
              <p className="truncate text-xs text-muted-foreground">{heading.description}</p>
            </div>
          </header>
          <div className="h-[calc(100vh-4rem)] overflow-y-auto">{children}</div>
        </SidebarInset>
      </div>
    </SidebarProvider>
  );
}

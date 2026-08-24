import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type SidebarContextValue = { collapsed: boolean; toggle: () => void };
const SidebarContext = React.createContext<SidebarContextValue | null>(null);

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [collapsed, setCollapsed] = React.useState(false);
  return <SidebarContext.Provider value={{ collapsed, toggle: () => setCollapsed((value) => !value) }}>{children}</SidebarContext.Provider>;
}

function useSidebar() {
  const value = React.useContext(SidebarContext);
  if (!value) throw new Error("Sidebar components require SidebarProvider");
  return value;
}

export function Sidebar({ className, ...props }: React.HTMLAttributes<HTMLElement>) {
  const { collapsed } = useSidebar();
  return <aside data-collapsed={collapsed} className={cn("group/sidebar flex h-screen w-60 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-200 data-[collapsed=true]:w-[68px]", className)} {...props} />;
}

export function SidebarHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex h-16 items-center px-4", className)} {...props} />;
}

export function SidebarContent({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-3", className)} {...props} />;
}

export function SidebarFooter({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("border-t border-sidebar-border p-3", className)} {...props} />;
}

export function SidebarMenuButton({ active, label, className, children, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean; label: string }) {
  return (
    <button title={label} aria-label={label} data-active={active} className={cn("flex h-10 w-full items-center gap-3 rounded-lg px-3 text-sm text-sidebar-foreground/70 outline-none transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground group-data-[collapsed=true]/sidebar:justify-center group-data-[collapsed=true]/sidebar:px-0 [&_svg]:size-4 [&_svg]:shrink-0", className)} {...props}>
      {children}
      <span className="truncate group-data-[collapsed=true]/sidebar:hidden">{label}</span>
    </button>
  );
}

export function SidebarTrigger({ className }: { className?: string }) {
  const { collapsed, toggle } = useSidebar();
  return (
    <Button variant="ghost" size="icon" className={className} onClick={toggle} aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}>
      {collapsed ? <PanelLeftOpen /> : <PanelLeftClose />}
    </Button>
  );
}

export function SidebarInset({ className, ...props }: React.HTMLAttributes<HTMLElement>) {
  return <main className={cn("min-w-0 flex-1 overflow-hidden bg-background", className)} {...props} />;
}

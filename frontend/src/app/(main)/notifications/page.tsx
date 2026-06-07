"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Notification } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Bell, CheckCheck } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

export default function NotificationsPage() {
  const { token, isAuthenticated, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.push("/login"); return; }
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated]);

  async function load() {
    try {
      const res = await api.notifications.list(token!, { page: 1 });
      setNotifications(res.notifications);
    } catch {
      toast.error("Could not load notifications");
    } finally {
      setLoading(false);
    }
  }

  async function markAllRead() {
    try {
      await api.notifications.markAllRead(token!);
      setNotifications((prev) => prev.map((n) => ({ ...n, is_read: true })));
      toast.success("All marked as read");
    } catch {
      toast.error("Failed to mark as read");
    }
  }

  async function markOneRead(id: number) {
    try {
      await api.notifications.markRead(id, token!);
      setNotifications((prev) =>
        prev.map((n) => (n.id === id ? { ...n, is_read: true } : n)),
      );
    } catch {
      // silent
    }
  }

  const unreadCount = notifications.filter((n) => !n.is_read).length;

  return (
    <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-foreground">Notifications</h1>
          {unreadCount > 0 && (
            <p className="text-sm text-muted-foreground mt-0.5">
              {unreadCount} unread
            </p>
          )}
        </div>
        {unreadCount > 0 && (
          <Button variant="outline" size="sm" onClick={markAllRead} className="gap-2">
            <CheckCheck className="h-4 w-4" />
            Mark all read
          </Button>
        )}
      </div>

      <div className="bg-white border border-border rounded-lg overflow-hidden">
        {loading ? (
          <div className="p-4 space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-16 rounded-lg" />
            ))}
          </div>
        ) : notifications.length === 0 ? (
          <div className="py-16 text-center">
            <Bell className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
            <p className="font-medium text-foreground">You&apos;re all caught up</p>
            <p className="text-sm text-muted-foreground mt-1">No notifications yet.</p>
          </div>
        ) : (
          notifications.map((n, i) => (
            <div key={n.id}>
              {i > 0 && <Separator />}
              <button
                type="button"
                onClick={() => !n.is_read && markOneRead(n.id)}
                className={cn(
                  "w-full text-left flex items-start gap-3 px-4 py-4 transition-colors hover:bg-muted/30",
                  !n.is_read && "bg-primary/5",
                )}
              >
                <div className="mt-0.5 shrink-0">
                  {!n.is_read ? (
                    <div className="h-2.5 w-2.5 rounded-full bg-primary mt-1" />
                  ) : (
                    <div className="h-2.5 w-2.5" />
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <p className={cn("text-sm", !n.is_read ? "font-semibold text-foreground" : "font-medium text-foreground")}>
                    {n.title}
                  </p>
                  <p className="text-sm text-muted-foreground mt-0.5">{n.body}</p>
                  <p className="text-xs text-muted-foreground mt-1">
                    {format(new Date(n.created_at), "d MMM yyyy, h:mm a")}
                  </p>
                </div>
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

"use client";

import { useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Booking, type BookingReceived, type DashboardData } from "@/lib/api";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { CalendarCheck, MapPin, Clock, User, MessageCircle } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";

const STATUS_COLORS: Record<string, string> = {
  pending: "bg-amber-100 text-amber-700 border-amber-200",
  confirmed: "bg-green-100 text-green-700 border-green-200",
  completed: "bg-blue-100 text-blue-700 border-blue-200",
  cancelled: "bg-red-100 text-red-700 border-red-200",
};

type ActionTarget = { id: number; label: string };
type MessageTarget = { clientId: number; targetType: string; targetId: number; clientName: string };

export default function BookingsPage() {
  const { token, isAuthenticated, user, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [bookings, setBookings] = useState<(Booking | BookingReceived)[]>([]);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<string>("all");
  const [dashboard, setDashboard] = useState<DashboardData | null>(null);
  const [actionLoading, setActionLoading] = useState<number | null>(null);

  // Cancel / reject dialog
  const [cancelTarget, setCancelTarget] = useState<ActionTarget | null>(null);
  const [cancelReason, setCancelReason] = useState("");

  // Complete confirmation dialog
  const [completeTarget, setCompleteTarget] = useState<ActionTarget | null>(null);

  // Message client dialog
  const [msgTarget, setMsgTarget] = useState<MessageTarget | null>(null);
  const [msgText, setMsgText] = useState("");
  const [msgSending, setMsgSending] = useState(false);

  const isProvider = user?.role === "provider";
  const isBusiness = user?.role === "business";
  const isServiceSide = isProvider || isBusiness;

  const loadBookings = useCallback(async (dash?: DashboardData | null) => {
    setLoading(true);
    try {
      if (isServiceSide) {
        const d = dash ?? dashboard;
        const targetType = isProvider ? "provider" : "business";
        const targetId = isProvider ? d?.provider_id : d?.business_id;
        if (!targetId) { setBookings([]); return; }
        const res = await api.bookings.received(token!, {
          target_type: targetType,
          target_id: targetId,
          status: tab === "all" ? undefined : tab,
        });
        setBookings(res.bookings);
      } else {
        const res = await api.bookings.myBookings(token!, {
          status: tab === "all" ? undefined : tab,
        });
        setBookings(res.bookings);
      }
    } catch {
      toast.error("Could not load bookings");
    } finally {
      setLoading(false);
    }
  }, [isServiceSide, isProvider, dashboard, token, tab]);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.push("/login"); return; }
    if (isServiceSide && !dashboard) {
      api.dashboard.get(token!).then((d) => {
        setDashboard(d);
        loadBookings(d);
      }).catch(() => toast.error("Could not load dashboard"));
    } else {
      loadBookings();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated, tab]);

  async function updateStatus(id: number, status: string, reason?: string) {
    setActionLoading(id);
    try {
      await api.bookings.updateStatus(id, status, token!, reason);
      toast.success(`Booking ${status}`);
      loadBookings();
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Action failed");
    } finally {
      setActionLoading(null);
    }
  }

  async function confirmCancel() {
    if (!cancelTarget) return;
    const target = cancelTarget;
    setCancelTarget(null);
    await updateStatus(target.id, "cancelled", cancelReason.trim() || undefined);
    setCancelReason("");
  }

  async function confirmComplete() {
    if (!completeTarget) return;
    const target = completeTarget;
    setCompleteTarget(null);
    await updateStatus(target.id, "completed");
  }

  async function sendMessage() {
    if (!msgTarget || !msgText.trim()) return;
    setMsgSending(true);
    try {
      await api.messages.send(
        {
          receiver_id: msgTarget.clientId,
          content: msgText.trim(),
          target_type: msgTarget.targetType,
          target_id: msgTarget.targetId,
        },
        token!,
      );
      toast.success("Message sent");
      setMsgTarget(null);
      setMsgText("");
      router.push("/messages");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Failed to send message");
    } finally {
      setMsgSending(false);
    }
  }

  const TABS = ["all", "pending", "confirmed", "completed", "cancelled"];

  function ActionButtons({ booking }: { booking: Booking }) {
    const busy = actionLoading === booking.id;
    const received = booking as BookingReceived;

    if (isServiceSide) {
      if (booking.status === "pending") {
        return (
          <div className="flex gap-2 shrink-0 flex-wrap justify-end">
            <Button size="sm" disabled={busy}
              onClick={() => updateStatus(booking.id, "confirmed")}>
              Confirm
            </Button>
            <Button size="sm" variant="outline" disabled={busy}
              className="text-destructive hover:text-destructive"
              onClick={() => { setCancelReason(""); setCancelTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}>
              Reject
            </Button>
          </div>
        );
      }
      if (booking.status === "confirmed") {
        return (
          <div className="flex gap-2 shrink-0 flex-wrap justify-end">
            <Button size="sm" variant="outline"
              onClick={() => {
                setMsgText("");
                setMsgTarget({
                  clientId: booking.client_id,
                  targetType: booking.target_type,
                  targetId: booking.target_id,
                  clientName: received.client_name ?? "Client",
                });
              }}
              className="gap-1.5"
            >
              <MessageCircle className="h-3.5 w-3.5" />
              Message
            </Button>
            <Button size="sm" disabled={busy}
              onClick={() => setCompleteTarget({ id: booking.id, label: `Booking #${booking.id}` })}>
              Complete
            </Button>
          </div>
        );
      }
      return null;
    }

    if (booking.status === "pending") {
      return (
        <Button size="sm" variant="outline" disabled={busy}
          className="shrink-0 text-destructive hover:text-destructive"
          onClick={() => { setCancelReason(""); setCancelTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}>
          Cancel
        </Button>
      );
    }
    return null;
  }

  const pageTitle = isServiceSide ? "Received Bookings" : "My Bookings";
  const emptyMessage = isServiceSide
    ? tab === "all" ? "No bookings received yet." : `No ${tab} bookings.`
    : tab === "all" ? "You haven't made any bookings yet." : `No ${tab} bookings.`;

  return (
    <>
      <div className="mx-auto max-w-3xl px-4 sm:px-6 py-8 space-y-6">
        <h1 className="text-2xl font-bold text-foreground">{pageTitle}</h1>

        <Tabs value={tab} onValueChange={setTab}>
          <TabsList className="bg-muted/50 overflow-x-auto flex-nowrap w-full justify-start">
            {TABS.map((t) => (
              <TabsTrigger key={t} value={t} className="capitalize text-xs shrink-0">{t}</TabsTrigger>
            ))}
          </TabsList>
        </Tabs>

        {loading ? (
          <div className="space-y-3">
            {Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} className="h-28 rounded-xl" />)}
          </div>
        ) : bookings.length === 0 ? (
          <div className="text-center py-16">
            <CalendarCheck className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
            <p className="font-medium text-foreground mb-1">No bookings found</p>
            <p className="text-sm text-muted-foreground mb-4">{emptyMessage}</p>
            {!isServiceSide && <Button onClick={() => router.push("/search")}>Find a provider</Button>}
          </div>
        ) : (
          <div className="space-y-3">
            {bookings.map((b) => {
              const received = b as BookingReceived;
              return (
                <Card key={b.id} className="border border-border">
                  <CardContent className="p-4">
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex-1 min-w-0 space-y-1.5">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="text-sm font-medium text-foreground">Booking #{b.id}</span>
                          <span className={`text-xs px-2 py-0.5 rounded-full border font-medium capitalize ${STATUS_COLORS[b.status] ?? "bg-muted text-muted-foreground border-border"}`}>
                            {b.status}
                          </span>
                        </div>

                        {isServiceSide && received.client_name && (
                          <div className="flex items-center gap-1 text-sm text-foreground">
                            <User className="h-3.5 w-3.5 text-muted-foreground" />
                            <span className="font-medium">{received.client_name}</span>
                            {received.client_email && (
                              <span className="text-muted-foreground text-xs">· {received.client_email}</span>
                            )}
                          </div>
                        )}

                        <p className="text-sm text-muted-foreground truncate">
                          {b.service_description ?? received.service_name ?? "Service booking"}
                        </p>

                        <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                          <span className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {format(new Date(b.scheduled_time), "d MMM yyyy, h:mm a")}
                          </span>
                          {b.client_address && (
                            <span className="flex items-center gap-1">
                              <MapPin className="h-3 w-3" />{b.client_address}
                            </span>
                          )}
                          {b.client_phone && <span className="text-xs">{b.client_phone}</span>}
                        </div>

                        {b.cancel_reason && (
                          <p className="text-xs text-destructive">Reason: {b.cancel_reason}</p>
                        )}
                      </div>

                      <ActionButtons booking={b} />
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}
      </div>

      {/* Cancel / Reject dialog */}
      <Dialog open={!!cancelTarget} onOpenChange={(open) => !open && setCancelTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{isServiceSide ? "Reject booking" : "Cancel booking"}</DialogTitle>
            <DialogDescription>
              {cancelTarget?.label} — optionally provide a reason.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <div className="space-y-1.5">
              <Label>Reason (optional)</Label>
              <Textarea
                placeholder={isServiceSide ? "e.g. Fully booked that day" : "e.g. Plans changed"}
                rows={3} value={cancelReason}
                onChange={(e) => setCancelReason(e.target.value)}
                className="resize-none"
              />
            </div>
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setCancelTarget(null)}>Keep booking</Button>
              <Button variant="destructive" onClick={confirmCancel}>
                {isServiceSide ? "Reject" : "Cancel booking"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Complete confirmation dialog */}
      <Dialog open={!!completeTarget} onOpenChange={(open) => !open && setCompleteTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Mark as completed?</DialogTitle>
            <DialogDescription>
              {completeTarget?.label} — confirm that you have fully completed this job before marking it done. This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <div className="flex gap-2 justify-end pt-2">
            <Button variant="outline" onClick={() => setCompleteTarget(null)}>Not yet</Button>
            <Button onClick={confirmComplete}>Yes, job is done</Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Message client dialog */}
      <Dialog open={!!msgTarget} onOpenChange={(open) => !open && setMsgTarget(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Message {msgTarget?.clientName}</DialogTitle>
            <DialogDescription>
              Send a message to your client about this booking.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <Textarea
              placeholder="e.g. Hi, I'm on my way — please have the area accessible."
              rows={5} value={msgText}
              onChange={(e) => setMsgText(e.target.value)}
              className="resize-none"
            />
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setMsgTarget(null)}>Cancel</Button>
              <Button onClick={sendMessage} disabled={msgSending || !msgText.trim()} className="gap-2">
                <MessageCircle className="h-4 w-4" />
                {msgSending ? "Sending…" : "Send"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

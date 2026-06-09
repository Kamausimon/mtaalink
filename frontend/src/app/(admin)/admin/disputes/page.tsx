"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type AdminDispute, type DisputeEvidence, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription,
} from "@/components/ui/dialog";
import { toast } from "sonner";
import { Gavel, CheckCircle2, XCircle, User, CalendarDays, MessageCircle, AlertTriangle, Clock, Ban } from "lucide-react";
import { format, parseISO } from "date-fns";
import { cn } from "@/lib/utils";

function fmtDate(iso: string | null) {
  if (!iso) return "—";
  try { return format(parseISO(iso), "d MMM yyyy, HH:mm"); } catch { return iso; }
}

type MsgTarget = { receiverId: number; name: string; targetType: string; targetId: number };

export default function AdminDisputesPage() {
  const { token, user, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [disputes, setDisputes] = useState<AdminDispute[]>([]);
  const [loading, setLoading] = useState(true);
  const [evidence, setEvidence] = useState<Record<number, DisputeEvidence[]>>({});

  // Resolve dialog
  const [selected, setSelected] = useState<AdminDispute | null>(null);
  const [resolution, setResolution] = useState<"completed" | "cancelled" | null>(null);
  const [note, setNote] = useState("");
  const [submitting, setSubmitting] = useState(false);

  // Message dialog
  const [msgTarget, setMsgTarget] = useState<MsgTarget | null>(null);
  const [msgText, setMsgText] = useState("");
  const [msgSending, setMsgSending] = useState(false);

  // Suspend dialog
  const [suspendTarget, setSuspendTarget] = useState<AdminDispute | null>(null);
  const [suspendDays, setSuspendDays] = useState<number>(7);
  const [suspending, setSuspending] = useState(false);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    api.admin.disputes(token)
      .then(async (r) => {
        setDisputes(r.disputes);
        // Fetch evidence for all disputes in parallel
        const evidenceMap: Record<number, DisputeEvidence[]> = {};
        await Promise.all(
          r.disputes.map(async (d) => {
            try {
              const ev = await api.bookings.getEvidence(d.booking_id, token!);
              evidenceMap[d.booking_id] = ev.evidence;
            } catch { /* non-critical */ }
          })
        );
        setEvidence(evidenceMap);
      })
      .catch((e) => {
        if (e instanceof ApiError && e.status === 403) router.replace("/dashboard");
        else toast.error("Failed to load disputes");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  function openResolve(dispute: AdminDispute, res: "completed" | "cancelled") {
    setSelected(dispute);
    setResolution(res);
    setNote("");
  }

  function closeResolve() { setSelected(null); setResolution(null); setNote(""); }

  function openMsg(target: MsgTarget) { setMsgTarget(target); setMsgText(""); }
  function closeMsg() { setMsgTarget(null); setMsgText(""); }

  async function handleResolve() {
    if (!selected || !resolution || !token) return;
    setSubmitting(true);
    try {
      await api.admin.resolveDispute(selected.booking_id, resolution, note.trim() || null, token);
      toast.success(
        resolution === "completed"
          ? "Dispute resolved — booking marked as completed. Both parties notified."
          : "Dispute resolved — booking cancelled. Both parties notified.",
      );
      setDisputes((d) => d.filter((x) => x.booking_id !== selected.booking_id));
      closeResolve();
    } catch {
      toast.error("Failed to resolve dispute");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleSuspend() {
    if (!suspendTarget || !token) return;
    const entityType = suspendTarget.target_type as "provider" | "business";
    const entityId = suspendTarget.target_id;
    setSuspending(true);
    try {
      await api.admin.suspend(entityType, entityId, suspendDays, token);
      const label = suspendDays === 0 ? "indefinitely" : `for ${suspendDays} day(s)`;
      toast.success(`${suspendTarget.provider_name ?? entityType} suspended ${label}`);
      setSuspendTarget(null);
    } catch {
      toast.error("Failed to suspend provider");
    } finally {
      setSuspending(false);
    }
  }

  async function handleSendMessage() {
    if (!msgTarget || !msgText.trim() || !token) return;
    setMsgSending(true);
    try {
      await api.messages.send(
        { receiver_id: msgTarget.receiverId, content: msgText.trim(), target_type: msgTarget.targetType, target_id: msgTarget.targetId },
        token,
      );
      toast.success(`Message sent to ${msgTarget.name}`);
      closeMsg();
    } catch {
      toast.error("Failed to send message");
    } finally {
      setMsgSending(false);
    }
  }

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-4 max-w-3xl">
        <Skeleton className="h-8 w-40" />
        {Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} className="h-52 rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-5 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Disputes</h1>
        <p className="text-sm text-muted-foreground mt-1">
          {disputes.length === 0
            ? "No active disputes."
            : `${disputes.length} dispute${disputes.length === 1 ? "" : "s"} awaiting mediation`}
        </p>
      </div>

      {disputes.length === 0 ? (
        <div className="rounded-xl border border-border bg-white px-6 py-12 text-center">
          <Gavel className="h-8 w-8 text-green-500 mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">All disputes resolved — nothing pending.</p>
        </div>
      ) : (
        <div className="space-y-5">
          {disputes.map((d) => (
            <div key={d.booking_id} className="rounded-xl border border-border bg-white overflow-hidden">
              {/* Header */}
              <div className="px-5 py-4 flex items-start justify-between gap-3 flex-wrap">
                <div>
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-base font-bold text-foreground">Booking #{d.booking_id}</span>
                    <span className="inline-flex items-center gap-1 text-xs font-semibold text-red-600 bg-red-50 rounded-full px-2 py-0.5">
                      <Gavel className="h-3 w-3" />Disputed
                    </span>
                    {!d.dispute_response && (
                      <span className="inline-flex items-center gap-1 text-xs font-semibold text-amber-600 bg-amber-50 rounded-full px-2 py-0.5">
                        <Clock className="h-3 w-3" />Awaiting provider response
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-muted-foreground mt-0.5">
                    {d.provider_name ?? `${d.target_type} #${d.target_id}`}
                    {d.service_description && <span className="ml-1 text-foreground/70">— {d.service_description}</span>}
                  </p>
                </div>
                <div className="flex gap-2 flex-wrap">
                  <Button size="sm" variant="outline"
                    className="text-green-700 border-green-200 hover:bg-green-50"
                    onClick={() => openResolve(d, "completed")}>
                    <CheckCircle2 className="h-4 w-4 mr-1" />Mark Completed
                  </Button>
                  <Button size="sm" variant="outline"
                    className="text-red-600 border-red-200 hover:bg-red-50"
                    onClick={() => openResolve(d, "cancelled")}>
                    <XCircle className="h-4 w-4 mr-1" />Cancel Booking
                  </Button>
                  <Button size="sm" variant="outline"
                    className="text-orange-700 border-orange-200 hover:bg-orange-50"
                    onClick={() => { setSuspendTarget(d); setSuspendDays(7); }}>
                    <Ban className="h-4 w-4 mr-1" />Suspend Provider
                  </Button>
                </div>
              </div>

              <Separator />

              {/* Parties */}
              <div className="px-5 py-3 grid grid-cols-1 sm:grid-cols-3 gap-3 text-sm bg-muted/20">
                <div className="flex items-center gap-2 text-muted-foreground">
                  <User className="h-4 w-4 shrink-0" />
                  <span>Client: <span className="font-medium text-foreground">{d.client_username}</span></span>
                </div>
                <div className="flex items-center gap-2 text-muted-foreground">
                  <CalendarDays className="h-4 w-4 shrink-0" />
                  <span className="font-medium text-foreground">{fmtDate(d.scheduled_time)}</span>
                </div>
                <div className="flex gap-2">
                  <Button size="sm" variant="ghost" className="h-7 gap-1.5 text-xs px-2"
                    onClick={() => openMsg({ receiverId: d.client_id, name: d.client_username, targetType: d.target_type, targetId: d.target_id })}>
                    <MessageCircle className="h-3.5 w-3.5" />Message client
                  </Button>
                  {d.service_owner_user_id && (
                    <Button size="sm" variant="ghost" className="h-7 gap-1.5 text-xs px-2"
                      onClick={() => openMsg({ receiverId: d.service_owner_user_id!, name: d.provider_name ?? "Provider", targetType: d.target_type, targetId: d.target_id })}>
                      <MessageCircle className="h-3.5 w-3.5" />Message provider
                    </Button>
                  )}
                </div>
              </div>

              <Separator />

              {/* Statements */}
              <div className="px-5 py-4 space-y-3">
                {/* Client side */}
                <div className="space-y-1.5">
                  <p className="text-xs font-semibold text-red-700 flex items-center gap-1.5">
                    <AlertTriangle className="h-3.5 w-3.5" />
                    Client&apos;s dispute reason:
                  </p>
                  {d.dispute_reason ? (
                    <div className="bg-red-50 border border-red-100 rounded-lg px-4 py-3 text-sm text-red-900">
                      {d.dispute_reason}
                    </div>
                  ) : (
                    <p className="text-sm text-muted-foreground italic">No reason provided.</p>
                  )}
                </div>

                {/* Provider side */}
                <div className="space-y-1.5">
                  <p className="text-xs font-semibold text-blue-700 flex items-center gap-1.5">
                    <User className="h-3.5 w-3.5" />
                    Provider&apos;s response:
                  </p>
                  {d.dispute_response ? (
                    <div className="bg-blue-50 border border-blue-100 rounded-lg px-4 py-3 text-sm text-blue-900">
                      {d.dispute_response}
                    </div>
                  ) : (
                    <div className="bg-muted/40 border border-border rounded-lg px-4 py-3 text-sm text-muted-foreground italic">
                      Provider has not responded yet. You can message them to request a response.
                    </div>
                  )}
                </div>
              </div>

              {/* Evidence */}
              {(evidence[d.booking_id]?.length ?? 0) > 0 && (
                <>
                  <Separator />
                  <div className="px-5 py-4 space-y-3">
                    <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Evidence submitted</p>
                    {/* Client evidence */}
                    {evidence[d.booking_id].filter(e => e.uploader_role === "client").length > 0 && (
                      <div className="space-y-2">
                        <p className="text-xs font-semibold text-red-700">From client:</p>
                        <div className="flex gap-2 flex-wrap">
                          {evidence[d.booking_id].filter(e => e.uploader_role === "client").map((ev) => (
                            <a key={ev.id} href={`http://localhost:7878${ev.file_url}`} target="_blank" rel="noreferrer" className="group relative">
                              <img
                                src={`http://localhost:7878${ev.file_url}`}
                                alt={ev.caption ?? "evidence"}
                                className="h-24 w-24 object-cover rounded-lg border-2 border-red-200 hover:border-red-400 transition-colors"
                              />
                              {ev.caption && (
                                <p className="text-xs text-muted-foreground mt-0.5 w-24 truncate">{ev.caption}</p>
                              )}
                            </a>
                          ))}
                        </div>
                      </div>
                    )}
                    {/* Provider evidence */}
                    {evidence[d.booking_id].filter(e => e.uploader_role === "provider").length > 0 && (
                      <div className="space-y-2">
                        <p className="text-xs font-semibold text-blue-700">From provider:</p>
                        <div className="flex gap-2 flex-wrap">
                          {evidence[d.booking_id].filter(e => e.uploader_role === "provider").map((ev) => (
                            <a key={ev.id} href={`http://localhost:7878${ev.file_url}`} target="_blank" rel="noreferrer" className="group relative">
                              <img
                                src={`http://localhost:7878${ev.file_url}`}
                                alt={ev.caption ?? "evidence"}
                                className="h-24 w-24 object-cover rounded-lg border-2 border-blue-200 hover:border-blue-400 transition-colors"
                              />
                              {ev.caption && (
                                <p className="text-xs text-muted-foreground mt-0.5 w-24 truncate">{ev.caption}</p>
                              )}
                            </a>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </>
              )}

              <div className="px-5 pb-3">
                <p className="text-xs text-muted-foreground">Raised: {fmtDate(d.created_at)}</p>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Resolve dialog */}
      <Dialog open={selected !== null} onOpenChange={(o) => !o && closeResolve()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {resolution === "completed" ? "Resolve as completed?" : "Cancel booking?"}
            </DialogTitle>
            <DialogDescription>
              {resolution === "completed"
                ? "Confirm the service was delivered. Both client and provider will be notified."
                : "Cancel the booking. The client and provider will both be notified with your decision."}
            </DialogDescription>
          </DialogHeader>

          {selected && (
            <div className="space-y-3 text-sm">
              {selected.dispute_reason && (
                <div className="bg-red-50 rounded-lg px-4 py-3">
                  <span className="font-semibold text-red-700">Client: </span>
                  <span className="text-red-900">{selected.dispute_reason}</span>
                </div>
              )}
              {selected.dispute_response && (
                <div className="bg-blue-50 rounded-lg px-4 py-3">
                  <span className="font-semibold text-blue-700">Provider: </span>
                  <span className="text-blue-900">{selected.dispute_response}</span>
                </div>
              )}
            </div>
          )}

          <div className="space-y-1.5">
            <Label htmlFor="admin-note">Decision note — shown to both parties</Label>
            <Textarea
              id="admin-note"
              rows={3}
              placeholder="Explain your ruling clearly for both the client and the provider…"
              value={note}
              onChange={(e) => setNote(e.target.value)}
            />
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={closeResolve} disabled={submitting}>Cancel</Button>
            <Button
              onClick={handleResolve}
              disabled={submitting}
              className={cn(resolution === "cancelled" && "bg-destructive text-destructive-foreground hover:bg-destructive/90")}
            >
              {submitting ? "Resolving…" : resolution === "completed" ? "Confirm — Mark Completed" : "Confirm — Cancel Booking"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Suspend dialog */}
      <Dialog open={suspendTarget !== null} onOpenChange={(o) => !o && setSuspendTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Ban className="h-4 w-4 text-orange-600" />
              Suspend {suspendTarget?.provider_name ?? suspendTarget?.target_type}
            </DialogTitle>
            <DialogDescription>
              The provider will be notified and cannot accept new bookings during the suspension period.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <p className="text-sm font-medium text-foreground">Suspension duration:</p>
            <div className="grid grid-cols-2 gap-2">
              {[1, 3, 7, 30].map((d) => (
                <Button
                  key={d}
                  variant={suspendDays === d ? "default" : "outline"}
                  size="sm"
                  onClick={() => setSuspendDays(d)}
                >
                  {d} day{d !== 1 ? "s" : ""}
                </Button>
              ))}
              <Button
                variant={suspendDays === 0 ? "default" : "outline"}
                size="sm"
                className={suspendDays === 0 ? "col-span-2 bg-red-600 hover:bg-red-700" : "col-span-2"}
                onClick={() => setSuspendDays(0)}
              >
                Permanent ban
              </Button>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSuspendTarget(null)} disabled={suspending}>Cancel</Button>
            <Button
              onClick={handleSuspend}
              disabled={suspending}
              className="bg-orange-600 hover:bg-orange-700 text-white"
            >
              {suspending ? "Suspending…" : "Confirm suspension"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Message dialog */}
      <Dialog open={msgTarget !== null} onOpenChange={(o) => !o && closeMsg()}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Message {msgTarget?.name}</DialogTitle>
            <DialogDescription>
              Send a message as admin to request more information or clarify the dispute.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <Textarea
              rows={4}
              placeholder="Type your message…"
              value={msgText}
              onChange={(e) => setMsgText(e.target.value)}
              className="resize-none"
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeMsg} disabled={msgSending}>Cancel</Button>
            <Button onClick={handleSendMessage} disabled={!msgText.trim() || msgSending}>
              {msgSending ? "Sending…" : "Send message"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

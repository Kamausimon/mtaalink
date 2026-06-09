"use client";

import { useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Booking, type BookingReceived, type DashboardData, ApiError } from "@/lib/api";
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
import { Input } from "@/components/ui/input";
import { CalendarCheck, MapPin, Clock, User, MessageCircle, CheckCircle2, AlertTriangle, Star, ImagePlus, X, Smartphone, Loader2 } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";

const STATUS_COLORS: Record<string, string> = {
  pending: "bg-amber-100 text-amber-700 border-amber-200",
  confirmed: "bg-green-100 text-green-700 border-green-200",
  completed: "bg-blue-100 text-blue-700 border-blue-200",
  cancelled: "bg-red-100 text-red-700 border-red-200",
  pending_confirmation: "bg-purple-100 text-purple-700 border-purple-200",
  disputed: "bg-rose-100 text-rose-700 border-rose-200",
};

function statusLabel(status: string, isServiceSide: boolean): string {
  if (status === "pending_confirmation") return isServiceSide ? "Awaiting client confirmation" : "Awaiting your confirmation";
  return status.replace(/_/g, " ");
}

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

  // Complete → pending_confirmation dialog (provider)
  const [completeTarget, setCompleteTarget] = useState<ActionTarget | null>(null);

  // Dispute dialog (client)
  const [disputeTarget, setDisputeTarget] = useState<ActionTarget | null>(null);
  const [disputeReason, setDisputeReason] = useState("");

  // Dispute response dialog (provider/business)
  const [disputeResponseTarget, setDisputeResponseTarget] = useState<ActionTarget | null>(null);
  const [disputeResponseText, setDisputeResponseText] = useState("");
  const [disputeResponseSending, setDisputeResponseSending] = useState(false);

  // Evidence upload dialog
  const [evidenceTarget, setEvidenceTarget] = useState<ActionTarget | null>(null);
  const [evidenceFiles, setEvidenceFiles] = useState<{ file: File; caption: string; preview: string }[]>([]);
  const [evidenceUploading, setEvidenceUploading] = useState(false);

  // M-Pesa payment dialog (client)
  const [payTarget, setPayTarget] = useState<{ bookingId: number; label: string } | null>(null);
  const [payPhone, setPayPhone] = useState("");
  const [payAmount, setPayAmount] = useState("");
  const [payStatus, setPayStatus] = useState<"idle" | "sending" | "polling" | "done" | "failed">("idle");
  const [payMessage, setPayMessage] = useState("");

  // Message client dialog (provider)
  const [msgTarget, setMsgTarget] = useState<MessageTarget | null>(null);
  const [msgText, setMsgText] = useState("");
  const [msgSending, setMsgSending] = useState(false);

  // Review dialog (client, completed bookings)
  type ReviewTarget = { bookingId: number; targetType: string; targetId: number; name: string };
  const [reviewTarget, setReviewTarget] = useState<ReviewTarget | null>(null);
  const [reviewRating, setReviewRating] = useState(0);
  const [reviewComment, setReviewComment] = useState("");
  const [reviewSending, setReviewSending] = useState(false);
  const [reviewedBookings, setReviewedBookings] = useState<Set<number>>(new Set());

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

  async function updateStatus(id: number, status: string, cancelReason?: string, disputeReason?: string) {
    setActionLoading(id);
    try {
      await api.bookings.updateStatus(id, status, token!, cancelReason, disputeReason);
      const labels: Record<string, string> = {
        confirmed: "Booking confirmed",
        cancelled: "Booking cancelled",
        pending_confirmation: "Marked as done — client will be notified to confirm",
        completed: "Job confirmed as complete",
        disputed: "Dispute raised — the provider has been notified",
      };
      toast.success(labels[status] ?? `Booking ${status}`);
      loadBookings();
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Action failed");
    } finally {
      setActionLoading(null);
    }
  }

  async function confirmCancel() {
    if (!cancelTarget) return;
    const t = cancelTarget; setCancelTarget(null);
    await updateStatus(t.id, "cancelled", cancelReason.trim() || undefined);
    setCancelReason("");
  }

  async function confirmComplete() {
    if (!completeTarget) return;
    const t = completeTarget; setCompleteTarget(null);
    await updateStatus(t.id, "pending_confirmation");
  }

  async function confirmDispute() {
    if (!disputeTarget) return;
    const t = disputeTarget; setDisputeTarget(null);
    await updateStatus(t.id, "disputed", undefined, disputeReason.trim() || undefined);
    setDisputeReason("");
  }

  async function confirmDone(id: number) {
    await updateStatus(id, "completed");
  }

  async function submitDisputeResponse() {
    if (!disputeResponseTarget || !disputeResponseText.trim()) return;
    setDisputeResponseSending(true);
    try {
      await api.bookings.submitDisputeResponse(disputeResponseTarget.id, disputeResponseText.trim(), token!);
      toast.success("Response submitted — the admin will review and mediate.");
      setDisputeResponseTarget(null);
      setDisputeResponseText("");
      loadBookings();
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Failed to submit response");
    } finally {
      setDisputeResponseSending(false);
    }
  }

  function addEvidenceFiles(files: FileList | null) {
    if (!files) return;
    const allowed = ["image/jpeg", "image/png", "image/webp", "image/heic"];
    const newFiles = Array.from(files)
      .filter((f) => allowed.includes(f.type))
      .slice(0, 5 - evidenceFiles.length)
      .map((file) => ({ file, caption: "", preview: URL.createObjectURL(file) }));
    setEvidenceFiles((prev) => [...prev, ...newFiles].slice(0, 5));
  }

  async function uploadEvidence() {
    if (!evidenceTarget || evidenceFiles.length === 0 || !token) return;
    setEvidenceUploading(true);
    let uploaded = 0;
    for (const { file, caption } of evidenceFiles) {
      try {
        await api.bookings.uploadEvidence(evidenceTarget.id, file, caption, token);
        uploaded++;
      } catch (err) {
        const msg = err instanceof ApiError ? err.message : "Upload failed";
        toast.error(`Failed to upload ${file.name}: ${msg}`);
      }
    }
    if (uploaded > 0) toast.success(`${uploaded} image${uploaded > 1 ? "s" : ""} uploaded as evidence`);
    setEvidenceTarget(null);
    evidenceFiles.forEach((f) => URL.revokeObjectURL(f.preview));
    setEvidenceFiles([]);
    setEvidenceUploading(false);
  }

  async function sendMessage() {
    if (!msgTarget || !msgText.trim()) return;
    setMsgSending(true);
    try {
      await api.messages.send(
        { receiver_id: msgTarget.clientId, content: msgText.trim(), target_type: msgTarget.targetType, target_id: msgTarget.targetId },
        token!,
      );
      toast.success("Message sent");
      setMsgTarget(null); setMsgText("");
      router.push("/messages");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Failed to send");
    } finally {
      setMsgSending(false);
    }
  }

  async function submitReview() {
    if (!reviewTarget || !reviewRating) return;
    setReviewSending(true);
    try {
      await api.reviews.create(
        { comment: reviewComment.trim(), rating: reviewRating },
        reviewTarget.targetType,
        reviewTarget.targetId,
        token!,
      );
      toast.success("Review submitted — thanks!");
      setReviewedBookings((prev) => new Set(prev).add(reviewTarget.bookingId));
      setReviewTarget(null); setReviewRating(0); setReviewComment("");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Could not submit review");
    } finally {
      setReviewSending(false);
    }
  }

  async function initiatePayment() {
    if (!payTarget || !payPhone.trim() || !payAmount || !token) return;
    const amount = parseFloat(payAmount);
    if (isNaN(amount) || amount <= 0) { toast.error("Enter a valid amount"); return; }
    setPayStatus("sending");
    setPayMessage("Sending M-Pesa prompt to your phone…");
    try {
      await api.payments.initiate({ booking_id: payTarget.bookingId, phone_number: payPhone.trim(), amount }, token);
      setPayStatus("polling");
      setPayMessage("Check your phone — enter your M-Pesa PIN to complete payment.");
      // Poll for up to 90 seconds
      let attempts = 0;
      const interval = setInterval(async () => {
        attempts++;
        try {
          const r = await api.payments.status(payTarget.bookingId, token!) as { payment: { status: string; transaction_id?: string } };
          if (r.payment.status === "completed") {
            clearInterval(interval);
            setPayStatus("done");
            setPayMessage(`Payment confirmed! M-Pesa receipt: ${r.payment.transaction_id ?? "N/A"}`);
            loadBookings();
          } else if (r.payment.status === "failed" || r.payment.status === "cancelled") {
            clearInterval(interval);
            setPayStatus("failed");
            setPayMessage("Payment failed or was cancelled. You can try again.");
          } else if (attempts >= 18) {
            clearInterval(interval);
            setPayStatus("failed");
            setPayMessage("Payment timed out. Check M-Pesa messages and try again if needed.");
          }
        } catch { /* ignore poll errors */ }
      }, 5000);
    } catch (e: unknown) {
      setPayStatus("failed");
      setPayMessage(e instanceof Error ? e.message : "Failed to initiate payment");
    }
  }

  const TABS = ["all", "pending", "confirmed", "pending_confirmation", "completed", "cancelled", "disputed"];
  const TAB_LABELS: Record<string, string> = { pending_confirmation: "Awaiting" };

  function ActionButtons({ booking }: { booking: Booking }) {
    const busy = actionLoading === booking.id;
    const received = booking as BookingReceived;

    if (isServiceSide) {
      if (booking.status === "pending") {
        return (
          <div className="flex gap-2 shrink-0 flex-wrap justify-end">
            <Button size="sm" disabled={busy} onClick={() => updateStatus(booking.id, "confirmed")}>Confirm</Button>
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
              onClick={() => { setMsgText(""); setMsgTarget({ clientId: booking.client_id, targetType: booking.target_type, targetId: booking.target_id, clientName: received.client_name ?? "Client" }); }}
              className="gap-1.5">
              <MessageCircle className="h-3.5 w-3.5" />Message
            </Button>
            <Button size="sm" disabled={busy}
              onClick={() => setCompleteTarget({ id: booking.id, label: `Booking #${booking.id}` })}>
              Mark done
            </Button>
          </div>
        );
      }
      if (booking.status === "disputed") {
        const alreadyResponded = !!(booking as Booking).dispute_response;
        return (
          <div className="flex gap-2 flex-wrap justify-end">
            <Button
              size="sm"
              variant="outline"
              disabled={alreadyResponded}
              className="text-amber-700 border-amber-200 hover:bg-amber-50"
              onClick={() => { setDisputeResponseText(""); setDisputeResponseTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}
            >
              {alreadyResponded ? "Response submitted" : "Respond to dispute"}
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="gap-1.5"
              onClick={() => { setEvidenceFiles([]); setEvidenceTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}
            >
              <ImagePlus className="h-3.5 w-3.5" />Add evidence
            </Button>
          </div>
        );
      }
      return null;
    }

    // Client side
    if (booking.status === "pending") {
      return (
        <div className="flex gap-2 shrink-0 flex-wrap justify-end">
          <Button size="sm" className="gap-1.5 bg-green-600 hover:bg-green-700 text-white"
            onClick={() => { setPayPhone(""); setPayAmount(""); setPayStatus("idle"); setPayMessage(""); setPayTarget({ bookingId: booking.id, label: `Booking #${booking.id}` }); }}>
            <Smartphone className="h-3.5 w-3.5" />Pay via M-Pesa
          </Button>
          <Button size="sm" variant="outline" disabled={busy}
            className="text-destructive hover:text-destructive"
            onClick={() => { setCancelReason(""); setCancelTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}>
            Cancel
          </Button>
        </div>
      );
    }
    if (booking.status === "confirmed") {
      return (
        <Button size="sm" className="gap-1.5 shrink-0 bg-green-600 hover:bg-green-700 text-white"
          onClick={() => { setPayPhone(""); setPayAmount(""); setPayStatus("idle"); setPayMessage(""); setPayTarget({ bookingId: booking.id, label: `Booking #${booking.id}` }); }}>
          <Smartphone className="h-3.5 w-3.5" />Pay via M-Pesa
        </Button>
      );
    }
    if (booking.status === "pending_confirmation") {
      return (
        <div className="flex gap-2 shrink-0 flex-wrap justify-end">
          <Button size="sm" disabled={busy} className="gap-1.5"
            onClick={() => confirmDone(booking.id)}>
            <CheckCircle2 className="h-3.5 w-3.5" />Confirm done
          </Button>
          <Button size="sm" variant="outline" disabled={busy}
            className="text-destructive hover:text-destructive gap-1.5"
            onClick={() => { setDisputeReason(""); setDisputeTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}>
            <AlertTriangle className="h-3.5 w-3.5" />Dispute
          </Button>
        </div>
      );
    }
    if (booking.status === "disputed") {
      return (
        <Button
          size="sm"
          variant="outline"
          className="shrink-0 gap-1.5"
          onClick={() => { setEvidenceFiles([]); setEvidenceTarget({ id: booking.id, label: `Booking #${booking.id}` }); }}
        >
          <ImagePlus className="h-3.5 w-3.5" />Add evidence
        </Button>
      );
    }
    if (booking.status === "completed" && !reviewedBookings.has(booking.id)) {
      return (
        <Button size="sm" variant="outline" className="shrink-0 gap-1.5"
          onClick={() => {
            setReviewRating(0); setReviewComment("");
            setReviewTarget({ bookingId: booking.id, targetType: booking.target_type, targetId: booking.target_id, name: (booking as BookingReceived).service_name ?? `Booking #${booking.id}` });
          }}>
          <Star className="h-3.5 w-3.5" />Leave a review
        </Button>
      );
    }
    return null;
  }

  const pageTitle = isServiceSide ? "Received Bookings" : "My Bookings";

  return (
    <>
      <div className="mx-auto max-w-3xl px-4 sm:px-6 py-8 space-y-6">
        <h1 className="text-2xl font-bold text-foreground">{pageTitle}</h1>

        <Tabs value={tab} onValueChange={setTab}>
          <TabsList className="bg-muted/50 overflow-x-auto flex-nowrap w-full justify-start">
            {TABS.map((t) => (
              <TabsTrigger key={t} value={t} className="capitalize text-xs shrink-0">
                {TAB_LABELS[t] ?? t}
              </TabsTrigger>
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
            {!isServiceSide && <Button className="mt-4" onClick={() => router.push("/search")}>Find a provider</Button>}
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
                            {statusLabel(b.status, isServiceSide)}
                          </span>
                        </div>

                        {/* Client info (provider/business view) */}
                        {isServiceSide && received.client_name && (
                          <div className="flex items-center gap-1 text-sm text-foreground">
                            <User className="h-3.5 w-3.5 text-muted-foreground" />
                            <span className="font-medium">{received.client_name}</span>
                            {received.client_email && (
                              <span className="text-muted-foreground text-xs">· {received.client_email}</span>
                            )}
                          </div>
                        )}

                        {/* Pending confirmation banner (client view) */}
                        {!isServiceSide && b.status === "pending_confirmation" && (
                          <p className="text-xs text-purple-700 bg-purple-50 border border-purple-200 rounded-md px-2 py-1.5">
                            The provider has marked this job as done. Please confirm if the work was completed or raise a dispute.
                          </p>
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
                            <span className="flex items-center gap-1"><MapPin className="h-3 w-3" />{b.client_address}</span>
                          )}
                          {b.client_phone && <span>{b.client_phone}</span>}
                        </div>

                        {b.cancel_reason && (
                          <p className="text-xs text-destructive">Reason: {b.cancel_reason}</p>
                        )}
                        {b.dispute_reason && (
                          <p className="text-xs text-rose-600">Dispute: {b.dispute_reason}</p>
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

      {/* Cancel / Reject */}
      <Dialog open={!!cancelTarget} onOpenChange={(open) => !open && setCancelTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{isServiceSide ? "Reject booking" : "Cancel booking"}</DialogTitle>
            <DialogDescription>{cancelTarget?.label} — optionally provide a reason.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <div className="space-y-1.5">
              <Label>Reason (optional)</Label>
              <Textarea placeholder={isServiceSide ? "e.g. Fully booked that day" : "e.g. Plans changed"}
                rows={3} value={cancelReason} onChange={(e) => setCancelReason(e.target.value)} className="resize-none" />
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

      {/* Mark done (provider → pending_confirmation) */}
      <Dialog open={!!completeTarget} onOpenChange={(open) => !open && setCompleteTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Mark job as done?</DialogTitle>
            <DialogDescription>
              {completeTarget?.label} — the client will be notified and asked to confirm the work is complete. You cannot undo this.
            </DialogDescription>
          </DialogHeader>
          <div className="flex gap-2 justify-end pt-2">
            <Button variant="outline" onClick={() => setCompleteTarget(null)}>Not yet</Button>
            <Button onClick={confirmComplete}>Yes, I&apos;m done</Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Dispute (client) */}
      <Dialog open={!!disputeTarget} onOpenChange={(open) => !open && setDisputeTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Raise a dispute</DialogTitle>
            <DialogDescription>
              {disputeTarget?.label} — describe what was not completed. The provider will be notified.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <div className="space-y-1.5">
              <Label>What went wrong? <span className="text-destructive">*</span></Label>
              <Textarea placeholder="e.g. The provider only completed half the job and left without finishing."
                rows={4} value={disputeReason} onChange={(e) => setDisputeReason(e.target.value)} className="resize-none" />
            </div>
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setDisputeTarget(null)}>Cancel</Button>
              <Button variant="destructive" disabled={!disputeReason.trim()} onClick={confirmDispute}>
                Submit dispute
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Respond to dispute (provider/business) */}
      <Dialog open={!!disputeResponseTarget} onOpenChange={(open) => !open && setDisputeResponseTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Respond to dispute</DialogTitle>
            <DialogDescription>
              {disputeResponseTarget?.label} — provide your side of the story. The admin will review both accounts before making a decision.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <div className="space-y-1.5">
              <Label>Your response <span className="text-destructive">*</span></Label>
              <Textarea
                placeholder="e.g. I completed all the agreed work. The client was present and did not raise any concerns during the job."
                rows={5}
                value={disputeResponseText}
                onChange={(e) => setDisputeResponseText(e.target.value)}
                className="resize-none"
              />
            </div>
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setDisputeResponseTarget(null)}>Cancel</Button>
              <Button
                disabled={!disputeResponseText.trim() || disputeResponseSending}
                onClick={submitDisputeResponse}
              >
                {disputeResponseSending ? "Submitting…" : "Submit response"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Evidence upload dialog */}
      <Dialog open={!!evidenceTarget} onOpenChange={(open) => { if (!open) { evidenceFiles.forEach(f => URL.revokeObjectURL(f.preview)); setEvidenceFiles([]); setEvidenceTarget(null); } }}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Upload evidence</DialogTitle>
            <DialogDescription>
              {evidenceTarget?.label} — add up to 5 photos to support your case. The admin will review all evidence from both parties.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            {/* File picker */}
            <label className="flex flex-col items-center justify-center gap-2 border-2 border-dashed border-border rounded-xl p-6 cursor-pointer hover:bg-muted/40 transition-colors">
              <ImagePlus className="h-7 w-7 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">Click to add photos ({evidenceFiles.length}/5)</span>
              <input
                type="file"
                accept="image/*"
                multiple
                className="hidden"
                disabled={evidenceFiles.length >= 5}
                onChange={(e) => addEvidenceFiles(e.target.files)}
              />
            </label>
            {/* Previews */}
            {evidenceFiles.length > 0 && (
              <div className="grid grid-cols-3 gap-2">
                {evidenceFiles.map((ef, i) => (
                  <div key={i} className="relative group">
                    <img src={ef.preview} alt="" className="w-full h-24 object-cover rounded-lg border border-border" />
                    <button
                      className="absolute top-1 right-1 bg-black/60 text-white rounded-full p-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
                      onClick={() => {
                        URL.revokeObjectURL(ef.preview);
                        setEvidenceFiles((prev) => prev.filter((_, idx) => idx !== i));
                      }}
                    >
                      <X className="h-3 w-3" />
                    </button>
                    <input
                      className="mt-1 w-full text-xs border border-border rounded px-1.5 py-1 bg-background"
                      placeholder="Caption…"
                      value={ef.caption}
                      onChange={(e) => setEvidenceFiles((prev) => prev.map((x, idx) => idx === i ? { ...x, caption: e.target.value } : x))}
                    />
                  </div>
                ))}
              </div>
            )}
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => { evidenceFiles.forEach(f => URL.revokeObjectURL(f.preview)); setEvidenceFiles([]); setEvidenceTarget(null); }}>
                Cancel
              </Button>
              <Button
                disabled={evidenceFiles.length === 0 || evidenceUploading}
                onClick={uploadEvidence}
              >
                {evidenceUploading ? "Uploading…" : `Upload ${evidenceFiles.length} image${evidenceFiles.length !== 1 ? "s" : ""}`}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Leave a review (client, completed) */}
      <Dialog open={!!reviewTarget} onOpenChange={(open) => !open && setReviewTarget(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Leave a review</DialogTitle>
            <DialogDescription>{reviewTarget?.name} — how was your experience?</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <div className="space-y-1.5">
              <Label>Rating <span className="text-destructive">*</span></Label>
              <div className="flex gap-1">
                {[1, 2, 3, 4, 5].map((n) => (
                  <button key={n} type="button" onClick={() => setReviewRating(n)}
                    className="p-0.5 focus:outline-none">
                    <Star className={`h-6 w-6 transition-colors ${n <= reviewRating ? "fill-amber-400 text-amber-400" : "text-muted-foreground"}`} />
                  </button>
                ))}
              </div>
            </div>
            <div className="space-y-1.5">
              <Label>Comment (optional)</Label>
              <Textarea placeholder="Tell others about your experience…"
                rows={4} value={reviewComment} onChange={(e) => setReviewComment(e.target.value)} className="resize-none" />
            </div>
            <div className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setReviewTarget(null)}>Cancel</Button>
              <Button disabled={!reviewRating || reviewSending} onClick={submitReview}>
                {reviewSending ? "Submitting…" : "Submit review"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Message client (provider) */}
      <Dialog open={!!msgTarget} onOpenChange={(open) => !open && setMsgTarget(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Message {msgTarget?.clientName}</DialogTitle>
            <DialogDescription>Send a message to your client about this booking.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <Textarea placeholder="e.g. Hi, I'm on my way — please have the area accessible."
              rows={5} value={msgText} onChange={(e) => setMsgText(e.target.value)} className="resize-none" />
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

      {/* M-Pesa payment dialog */}
      <Dialog open={!!payTarget} onOpenChange={(open) => { if (!open && payStatus !== "sending" && payStatus !== "polling") setPayTarget(null); }}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Smartphone className="h-5 w-5 text-green-600" />Pay via M-Pesa
            </DialogTitle>
            <DialogDescription>{payTarget?.label}</DialogDescription>
          </DialogHeader>

          {payStatus === "idle" && (
            <div className="space-y-4 pt-1">
              <div className="space-y-1.5">
                <Label>M-Pesa phone number <span className="text-destructive">*</span></Label>
                <Input
                  type="tel"
                  placeholder="07XX XXX XXX or 2547XX..."
                  value={payPhone}
                  onChange={(e) => setPayPhone(e.target.value)}
                />
              </div>
              <div className="space-y-1.5">
                <Label>Amount (KES) <span className="text-destructive">*</span></Label>
                <Input
                  type="number"
                  placeholder="e.g. 1500"
                  min="1"
                  value={payAmount}
                  onChange={(e) => setPayAmount(e.target.value)}
                />
              </div>
              <p className="text-xs text-muted-foreground">
                You will receive a PIN prompt on your phone. The provider&apos;s wallet is credited only after payment is confirmed by Safaricom.
              </p>
              <div className="flex gap-2 justify-end">
                <Button variant="outline" onClick={() => setPayTarget(null)}>Cancel</Button>
                <Button
                  className="bg-green-600 hover:bg-green-700 text-white gap-1.5"
                  disabled={!payPhone.trim() || !payAmount}
                  onClick={initiatePayment}
                >
                  <Smartphone className="h-4 w-4" />Send M-Pesa prompt
                </Button>
              </div>
            </div>
          )}

          {(payStatus === "sending" || payStatus === "polling") && (
            <div className="py-6 flex flex-col items-center gap-4 text-center">
              <Loader2 className="h-10 w-10 text-green-600 animate-spin" />
              <p className="text-sm font-medium text-foreground">{payMessage}</p>
              {payStatus === "polling" && (
                <p className="text-xs text-muted-foreground">Waiting for Safaricom confirmation… this can take up to 90 seconds.</p>
              )}
            </div>
          )}

          {payStatus === "done" && (
            <div className="py-6 flex flex-col items-center gap-3 text-center">
              <CheckCircle2 className="h-12 w-12 text-green-600" />
              <p className="text-sm font-semibold text-green-700">Payment successful!</p>
              <p className="text-xs text-muted-foreground">{payMessage}</p>
              <Button className="mt-2" onClick={() => setPayTarget(null)}>Close</Button>
            </div>
          )}

          {payStatus === "failed" && (
            <div className="py-4 flex flex-col items-center gap-3 text-center">
              <AlertTriangle className="h-10 w-10 text-destructive" />
              <p className="text-sm text-destructive font-medium">{payMessage}</p>
              <div className="flex gap-2">
                <Button variant="outline" onClick={() => setPayTarget(null)}>Close</Button>
                <Button onClick={() => { setPayStatus("idle"); setPayMessage(""); }}>Try again</Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}

"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type AdminPayout, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { CheckCircle2, XCircle } from "lucide-react";

function fmtKES(n: string | number) {
  return `KES ${Number(n).toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
}

export default function AdminPayoutsPage() {
  const { token, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [payouts, setPayouts] = useState<AdminPayout[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionId, setActionId] = useState<number | null>(null);
  const [actionType, setActionType] = useState<"approve" | "reject" | null>(null);
  const [notes, setNotes] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    api.admin.payouts(token)
      .then((r) => setPayouts(r.pending_payouts))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 403) router.replace("/dashboard");
        else toast.error("Failed to load payouts");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  function openAction(id: number, type: "approve" | "reject") {
    setActionId(id);
    setActionType(type);
    setNotes("");
  }

  function closeAction() {
    setActionId(null);
    setActionType(null);
    setNotes("");
  }

  async function handleSubmit() {
    if (!actionId || !actionType || !token) return;
    setSubmitting(true);
    try {
      if (actionType === "approve") {
        await api.admin.approvePayout(actionId, notes.trim() || null, token);
        toast.success("Payout approved");
      } else {
        await api.admin.rejectPayout(actionId, notes.trim() || null, token);
        toast.success("Payout rejected — balance refunded to wallet");
      }
      setPayouts((p) => p.filter((x) => x.id !== actionId));
      closeAction();
    } catch {
      toast.error("Action failed");
    } finally {
      setSubmitting(false);
    }
  }

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-4 max-w-3xl">
        <Skeleton className="h-8 w-32" />
        {Array.from({ length: 5 }).map((_, i) => <Skeleton key={i} className="h-20 rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-5 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Pending Payouts</h1>
        <p className="text-sm text-muted-foreground mt-1">
          {payouts.length === 0 ? "No pending payouts." : `${payouts.length} payout${payouts.length === 1 ? "" : "s"} awaiting approval`}
        </p>
      </div>

      {payouts.length === 0 ? (
        <div className="rounded-xl border border-border bg-white px-6 py-12 text-center">
          <CheckCircle2 className="h-8 w-8 text-green-500 mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">All payouts processed — nothing pending.</p>
        </div>
      ) : (
        <div className="rounded-xl border border-border overflow-hidden bg-white">
          {payouts.map((p, i) => (
            <div key={p.id}>
              {i > 0 && <Separator />}
              <div className="flex items-center justify-between gap-4 px-5 py-4">
                <div className="min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-base font-bold text-foreground">{fmtKES(p.amount)}</span>
                    <span className="text-xs text-muted-foreground bg-muted rounded px-1.5 py-0.5">
                      {p.target_type ?? "unknown"} #{p.target_id}
                    </span>
                  </div>
                  <p className="text-sm text-muted-foreground mt-0.5">
                    Mpesa: <span className="font-medium text-foreground">{p.phone_number}</span>
                  </p>
                  {p.notes && (
                    <p className="text-xs text-muted-foreground mt-1 italic">{p.notes}</p>
                  )}
                </div>
                <div className="flex gap-2 shrink-0">
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-green-700 border-green-200 hover:bg-green-50"
                    onClick={() => openAction(p.id, "approve")}
                  >
                    <CheckCircle2 className="h-4 w-4 mr-1" />
                    Approve
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-red-600 border-red-200 hover:bg-red-50"
                    onClick={() => openAction(p.id, "reject")}
                  >
                    <XCircle className="h-4 w-4 mr-1" />
                    Reject
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <Dialog open={actionId !== null} onOpenChange={(o) => !o && closeAction()}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {actionType === "approve" ? "Approve payout?" : "Reject payout?"}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-3 py-2">
            <p className="text-sm text-muted-foreground">
              {actionType === "approve"
                ? "Confirm that you have sent the M-Pesa payment."
                : "The payout amount will be refunded back to the provider's wallet."}
            </p>
            <div className="space-y-1.5">
              <Label htmlFor="notes">Notes (optional)</Label>
              <Input
                id="notes"
                placeholder={actionType === "approve" ? "Transaction ID or notes…" : "Reason for rejection…"}
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeAction} disabled={submitting}>
              Cancel
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={submitting}
              className={actionType === "reject" ? "bg-destructive text-destructive-foreground hover:bg-destructive/90" : ""}
            >
              {submitting ? "Processing…" : actionType === "approve" ? "Confirm Approval" : "Confirm Rejection"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

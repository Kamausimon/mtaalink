"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Wallet, type Transaction } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Wallet as WalletIcon, TrendingUp, ArrowDownCircle } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";

export default function WalletPage() {
  const { token, user, isAuthenticated } = useAuthStore();
  const router = useRouter();
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [targetId, setTargetId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [payoutOpen, setPayoutOpen] = useState(false);
  const [payoutForm, setPayoutForm] = useState({ amount: "", phone_number: "" });
  const [payoutLoading, setPayoutLoading] = useState(false);

  const targetType = user?.role === "business" ? "business" : "provider";

  useEffect(() => {
    if (!isAuthenticated) { router.push("/login"); return; }
    if (user?.role === "client") { router.push("/dashboard"); return; }
    loadWallet();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated, user]);

  async function loadWallet() {
    try {
      const dash = await api.dashboard.get(token!);
      const id = dash.provider_id ?? dash.business_id;
      if (!id) return;
      setTargetId(id);

      const [walletRes, txRes] = await Promise.all([
        api.wallet.get(targetType, id, token!),
        api.wallet.transactions(targetType, id, token!),
      ]);
      setWallet(walletRes.wallet);
      setTransactions(txRes.transactions);
    } catch {
      toast.error("Could not load wallet");
    } finally {
      setLoading(false);
    }
  }

  async function requestPayout() {
    const amount = Number(payoutForm.amount);
    if (!amount || amount <= 0) { toast.error("Enter a valid amount"); return; }
    if (!payoutForm.phone_number) { toast.error("Enter your M-Pesa phone number"); return; }
    if (!targetId) return;

    setPayoutLoading(true);
    try {
      await api.wallet.requestPayout(targetType, targetId, {
        amount,
        phone_number: payoutForm.phone_number,
      }, token!);

      toast.success("Payout request submitted. An admin will process it shortly.");
      setPayoutOpen(false);
      setPayoutForm({ amount: "", phone_number: "" });
      loadWallet();
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Payout request failed");
    } finally {
      setPayoutLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-4">
        <Skeleton className="h-10 w-32" />
        <div className="grid grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} className="h-28 rounded-xl" />)}
        </div>
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  const balance = wallet ? Number(wallet.balance) : 0;
  const totalEarned = wallet ? Number(wallet.total_earned) : 0;
  const totalPaidOut = wallet ? Number(wallet.total_paid_out) : 0;

  return (
    <>
      <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold text-foreground">Wallet</h1>
          <Button onClick={() => setPayoutOpen(true)} disabled={balance <= 0} className="gap-2">
            <ArrowDownCircle className="h-4 w-4" />
            Request payout
          </Button>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4">
          <Card className="border border-border">
            <CardContent className="p-4 space-y-1">
              <WalletIcon className="h-5 w-5 text-primary" />
              <p className="text-xl font-bold text-foreground">
                KES {balance.toLocaleString()}
              </p>
              <p className="text-xs text-muted-foreground">Available balance</p>
            </CardContent>
          </Card>
          <Card className="border border-border">
            <CardContent className="p-4 space-y-1">
              <TrendingUp className="h-5 w-5 text-primary" />
              <p className="text-xl font-bold text-foreground">
                KES {totalEarned.toLocaleString()}
              </p>
              <p className="text-xs text-muted-foreground">Total earned</p>
            </CardContent>
          </Card>
          <Card className="border border-border">
            <CardContent className="p-4 space-y-1">
              <ArrowDownCircle className="h-5 w-5 text-muted-foreground" />
              <p className="text-xl font-bold text-foreground">
                KES {totalPaidOut.toLocaleString()}
              </p>
              <p className="text-xs text-muted-foreground">Total paid out</p>
            </CardContent>
          </Card>
        </div>

        {/* Transaction history */}
        <Card className="border border-border">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">Transaction history</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            {transactions.length === 0 ? (
              <p className="text-sm text-muted-foreground px-5 py-6">No transactions yet.</p>
            ) : (
              transactions.map((tx, i) => (
                <div key={tx.id}>
                  {i > 0 && <Separator />}
                  <div className="flex items-center justify-between px-5 py-4">
                    <div>
                      <p className="text-sm font-medium text-foreground">{tx.description}</p>
                      <p className="text-xs text-muted-foreground">
                        {format(new Date(tx.created_at), "d MMM yyyy, h:mm a")}
                      </p>
                    </div>
                    <span className="text-sm font-semibold text-primary">
                      + KES {Number(tx.amount).toLocaleString()}
                    </span>
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>

      {/* Payout dialog */}
      <Dialog open={payoutOpen} onOpenChange={setPayoutOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Request payout</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 pt-2">
            <p className="text-sm text-muted-foreground">
              Available: <span className="font-semibold text-foreground">KES {balance.toLocaleString()}</span>
            </p>
            <div className="space-y-1.5">
              <Label>Amount (KES)</Label>
              <Input
                type="number"
                placeholder={`Max ${balance}`}
                max={balance}
                value={payoutForm.amount}
                onChange={(e) => setPayoutForm((f) => ({ ...f, amount: e.target.value }))}
              />
            </div>
            <div className="space-y-1.5">
              <Label>M-Pesa phone number</Label>
              <Input
                type="tel"
                placeholder="07XX XXX XXX"
                value={payoutForm.phone_number}
                onChange={(e) => setPayoutForm((f) => ({ ...f, phone_number: e.target.value }))}
              />
            </div>
            <Button className="w-full" onClick={requestPayout} disabled={payoutLoading}>
              {payoutLoading ? "Submitting…" : "Submit payout request"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

"use client";

import { useAuthStore } from "@/store/auth";
import { useState } from "react";
import { api } from "@/lib/api";
import { MailWarning, X, Loader2 } from "lucide-react";
import { toast } from "sonner";

export default function EmailVerificationBanner() {
  const { user, token, _hasHydrated } = useAuthStore();
  const [dismissed, setDismissed] = useState(false);
  const [sending, setSending] = useState(false);

  // Only show for authenticated users with unverified emails
  if (!_hasHydrated || !user || user.email_verified !== false || dismissed) return null;

  async function resend() {
    if (!token) return;
    setSending(true);
    try {
      // Re-register doesn't work; call a resend endpoint or use forgot-password flow.
      // For now, call a lightweight resend endpoint if it exists, otherwise guide user.
      await fetch(`${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:7878"}/auth/resend-verification`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
      });
      toast.success("Verification email sent — check your inbox");
    } catch {
      toast.error("Failed to resend. Try again shortly.");
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="bg-amber-50 border-b border-amber-200 px-4 py-2.5 flex items-center gap-3 text-sm text-amber-800">
      <MailWarning className="h-4 w-4 shrink-0" />
      <span className="flex-1">
        Please verify your email address.{" "}
        <button
          onClick={resend}
          disabled={sending}
          className="underline font-medium hover:text-amber-900 disabled:opacity-50 inline-flex items-center gap-1"
        >
          {sending && <Loader2 className="h-3 w-3 animate-spin" />}
          Resend verification email
        </button>
      </span>
      <button
        onClick={() => setDismissed(true)}
        className="text-amber-600 hover:text-amber-900 shrink-0"
        aria-label="Dismiss"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

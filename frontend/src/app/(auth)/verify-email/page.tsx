"use client";

import { useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import { api } from "@/lib/api";
import { CheckCircle2, XCircle, Loader2 } from "lucide-react";
import { Suspense } from "react";

function VerifyEmailContent() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token") ?? "";
  const [status, setStatus] = useState<"loading" | "success" | "error">("loading");
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (!token) {
      setStatus("error");
      setMessage("No verification token found. Check your email for the verification link.");
      return;
    }
    api.auth.verifyEmail(token)
      .then(() => setStatus("success"))
      .catch((e) => {
        setStatus("error");
        setMessage(e?.message ?? "Verification failed. The link may have expired.");
      });
  }, [token]);

  return (
    <div className="min-h-screen flex items-center justify-center bg-background px-4">
      <div className="w-full max-w-sm space-y-6 text-center">
        <span className="text-2xl font-bold text-primary tracking-tight">
          Mtaa<span className="text-accent">Link</span>
        </span>

        {status === "loading" && (
          <div className="rounded-xl border border-border bg-white px-6 py-10 space-y-3">
            <Loader2 className="h-10 w-10 text-primary mx-auto animate-spin" />
            <p className="font-medium text-foreground">Verifying your email…</p>
          </div>
        )}

        {status === "success" && (
          <div className="rounded-xl border border-border bg-white px-6 py-10 space-y-4">
            <CheckCircle2 className="h-12 w-12 text-green-500 mx-auto" />
            <div>
              <p className="font-semibold text-foreground text-lg">Email verified!</p>
              <p className="text-sm text-muted-foreground mt-1">
                Your email address has been confirmed. You can now use all features of MtaaLink.
              </p>
            </div>
            <Link href="/dashboard" className="inline-flex items-center justify-center w-full rounded-md bg-primary text-white text-sm font-medium h-10 px-4 py-2 hover:bg-primary/90 transition-colors">
              Go to dashboard
            </Link>
          </div>
        )}

        {status === "error" && (
          <div className="rounded-xl border border-border bg-white px-6 py-10 space-y-4">
            <XCircle className="h-12 w-12 text-destructive mx-auto" />
            <div>
              <p className="font-semibold text-foreground text-lg">Verification failed</p>
              <p className="text-sm text-muted-foreground mt-1">{message}</p>
            </div>
            <Link href="/dashboard" className="inline-flex items-center justify-center w-full rounded-md border border-border text-sm font-medium h-10 px-4 py-2 hover:bg-muted transition-colors">
              Go to dashboard
            </Link>
          </div>
        )}
      </div>
    </div>
  );
}

export default function VerifyEmailPage() {
  return (
    <Suspense>
      <VerifyEmailContent />
    </Suspense>
  );
}

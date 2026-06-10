"use client";

import { useState } from "react";
import Link from "next/link";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { Mail, ArrowLeft, CheckCircle2 } from "lucide-react";

export default function ForgotPasswordPage() {
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [sent, setSent] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!email.trim()) return;
    setLoading(true);
    try {
      await api.auth.forgotPassword(email.trim());
      setSent(true);
    } catch {
      // Always show success to avoid email enumeration
      setSent(true);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-background px-4">
      <div className="w-full max-w-sm space-y-6">
        <div className="text-center space-y-1">
          <span className="text-2xl font-bold text-primary tracking-tight">
            Sok<span className="text-accent">avi</span>
          </span>
          <h1 className="text-xl font-semibold text-foreground">Reset your password</h1>
          <p className="text-sm text-muted-foreground">
            Enter your email and we&apos;ll send you a reset link.
          </p>
        </div>

        {sent ? (
          <div className="rounded-xl border border-border bg-white px-6 py-8 text-center space-y-3">
            <CheckCircle2 className="h-10 w-10 text-green-500 mx-auto" />
            <p className="font-medium text-foreground">Check your email</p>
            <p className="text-sm text-muted-foreground">
              If <span className="font-medium text-foreground">{email}</span> is registered,
              you&apos;ll receive a reset link within a minute. Check your spam folder if you don&apos;t see it.
            </p>
            <p className="text-xs text-muted-foreground">The link expires in 15 minutes.</p>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="rounded-xl border border-border bg-white px-6 py-6 space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="email">Email address</Label>
              <div className="relative">
                <Mail className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  id="email"
                  type="email"
                  placeholder="you@example.com"
                  className="pl-9"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  required
                  autoFocus
                />
              </div>
            </div>
            <Button type="submit" className="w-full" disabled={loading || !email.trim()}>
              {loading ? "Sending…" : "Send reset link"}
            </Button>
          </form>
        )}

        <p className="text-center text-sm text-muted-foreground">
          <Link href="/login" className="inline-flex items-center gap-1 text-primary hover:underline">
            <ArrowLeft className="h-3.5 w-3.5" />Back to login
          </Link>
        </p>
      </div>
    </div>
  );
}

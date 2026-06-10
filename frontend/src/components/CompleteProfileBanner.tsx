"use client";

import { useAuthStore } from "@/store/auth";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { ClipboardList } from "lucide-react";

export default function CompleteProfileBanner() {
  const { user, _hasHydrated } = useAuthStore();
  const pathname = usePathname();

  if (!_hasHydrated || !user) return null;
  if (user.role !== "provider" && user.role !== "business") return null;
  if (user.onboarding_completed !== false) return null;
  if (pathname?.startsWith("/onboard")) return null;

  return (
    <div className="bg-primary/5 border-b border-primary/20 px-4 py-2.5 flex items-center gap-3 text-sm text-primary">
      <ClipboardList className="h-4 w-4 shrink-0" />
      <span className="flex-1">
        Your {user.role} profile isn&apos;t set up yet — clients can&apos;t find or book you until it&apos;s complete.
      </span>
      <Link
        href={`/onboard/${user.role}`}
        className="underline font-medium hover:text-primary/80 shrink-0"
      >
        Complete your profile
      </Link>
    </div>
  );
}

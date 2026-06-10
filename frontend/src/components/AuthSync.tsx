"use client";

import { useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api } from "@/lib/api";

export default function AuthSync() {
  const { user, token, _hasHydrated, updateUser } = useAuthStore();
  const pathname = usePathname();
  const router = useRouter();

  // Refresh user state from the backend so email_verified / onboarding_completed
  // reflect server truth instead of the value cached at login time.
  useEffect(() => {
    if (!_hasHydrated || !token || !user) return;

    let cancelled = false;
    api.auth
      .me(token)
      .then((fresh) => {
        if (cancelled) return;
        if (
          fresh.email_verified !== user.email_verified ||
          fresh.onboarding_completed !== user.onboarding_completed
        ) {
          updateUser({
            ...user,
            email_verified: fresh.email_verified,
            onboarding_completed: fresh.onboarding_completed,
          });
        }
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, token]);

  // Redirect provider/business accounts that haven't finished onboarding.
  useEffect(() => {
    if (!_hasHydrated || !user) return;
    if (user.role !== "provider" && user.role !== "business") return;
    if (user.onboarding_completed !== false) return;
    if (pathname?.startsWith("/onboard")) return;

    router.replace(`/onboard/${user.role}`);
  }, [_hasHydrated, user, pathname, router]);

  return null;
}

"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { hasSeenTour, startSiteTour } from "@/lib/tour";

export default function SiteTour() {
  const { user, _hasHydrated } = useAuthStore();
  const pathname = usePathname();

  useEffect(() => {
    if (!_hasHydrated || !user) return;
    if (pathname !== "/dashboard") return;
    if (hasSeenTour()) return;

    const timer = setTimeout(() => startSiteTour(user.role), 600);
    return () => clearTimeout(timer);
  }, [_hasHydrated, user, pathname]);

  return null;
}

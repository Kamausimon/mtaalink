"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api } from "@/lib/api";
import { Loader2 } from "lucide-react";

export default function MyProfilePage() {
  const router = useRouter();
  const { token, isAuthenticated, user, _hasHydrated } = useAuthStore();

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.replace("/login"); return; }
    if (user?.role === "client") { router.replace("/dashboard"); return; }

    api.dashboard.get(token!).then((d) => {
      if (d.role === "provider" && d.provider_id) {
        router.replace(`/providers/${d.provider_id}`);
      } else if (d.role === "business" && d.business_id) {
        router.replace(`/businesses/${d.business_id}`);
      } else {
        router.replace("/dashboard");
      }
    }).catch(() => router.replace("/dashboard"));
  }, [_hasHydrated, isAuthenticated, user?.role, token, router]);

  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
    </div>
  );
}

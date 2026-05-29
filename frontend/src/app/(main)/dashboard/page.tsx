"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type DashboardData } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import {
  CalendarCheck,
  Bell,
  Wallet,
  Clock,
  Search,
  ArrowRight,
} from "lucide-react";
import Link from "next/link";

export default function DashboardPage() {
  const { token, user, isAuthenticated } = useAuthStore();
  const router = useRouter();
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!isAuthenticated) {
      router.push("/login");
      return;
    }
    api.dashboard
      .get(token!)
      .then(setData)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [isAuthenticated, token, router]);

  if (loading) {
    return (
      <div className="mx-auto max-w-4xl px-4 sm:px-6 py-8 space-y-4">
        <Skeleton className="h-10 w-48" />
        <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-28 rounded-xl" />
          ))}
        </div>
      </div>
    );
  }

  if (!data) return null;

  const isProvider = user?.role === "provider";
  const isBusiness = user?.role === "business";
  const isClient = user?.role === "client";

  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 py-8 space-y-8">
      {/* Greeting */}
      <div>
        <h1 className="text-2xl font-bold text-foreground">
          Welcome back, {data.username}
        </h1>
        <p className="text-muted-foreground text-sm mt-1">
          Here&apos;s what&apos;s happening today.
        </p>
      </div>

      {/* Stats */}
      <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={<CalendarCheck className="h-5 w-5 text-primary" />}
          label="Upcoming bookings"
          value={String(data.upcoming_bookings)}
        />
        <StatCard
          icon={<Bell className="h-5 w-5 text-primary" />}
          label="Unread notifications"
          value={String(data.unread_notifications)}
          badge={data.unread_notifications > 0 ? "new" : undefined}
        />
        {(isProvider || isBusiness) && data.pending_bookings != null && (
          <StatCard
            icon={<Clock className="h-5 w-5 text-accent" />}
            label="Pending bookings"
            value={String(data.pending_bookings)}
            badge={data.pending_bookings > 0 ? "action needed" : undefined}
          />
        )}
        {(isProvider || isBusiness) && data.balance != null && (
          <StatCard
            icon={<Wallet className="h-5 w-5 text-primary" />}
            label="Wallet balance"
            value={`KES ${Number(data.balance).toLocaleString()}`}
          />
        )}
      </div>

      {/* Quick actions */}
      <div>
        <h2 className="text-base font-semibold text-foreground mb-3">
          Quick actions
        </h2>
        <div className="grid sm:grid-cols-2 gap-3">
          {isClient && (
            <ActionCard
              href="/search"
              icon={<Search className="h-5 w-5" />}
              title="Find a service provider"
              description="Search plumbers, cleaners, tutors and more near you"
            />
          )}
          <ActionCard
            href="/bookings"
            icon={<CalendarCheck className="h-5 w-5" />}
            title={isClient ? "My bookings" : "Manage bookings"}
            description={
              isClient
                ? "View and manage your service bookings"
                : "See pending and confirmed bookings"
            }
          />
          <ActionCard
            href="/messages"
            icon={<Bell className="h-5 w-5" />}
            title="Messages"
            description="Chat with providers or clients"
          />
          {(isProvider || isBusiness) && (
            <ActionCard
              href="/wallet"
              icon={<Wallet className="h-5 w-5" />}
              title="Wallet &amp; earnings"
              description="View your earnings and request a payout"
            />
          )}
        </div>
      </div>
    </div>
  );
}

function StatCard({
  icon,
  label,
  value,
  badge,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  badge?: string;
}) {
  return (
    <Card className="border border-border">
      <CardContent className="p-5 flex flex-col gap-3">
        <div className="flex items-center justify-between">
          {icon}
          {badge && (
            <Badge className="text-xs bg-accent text-white border-0 py-0">
              {badge}
            </Badge>
          )}
        </div>
        <div>
          <p className="text-2xl font-bold text-foreground">{value}</p>
          <p className="text-xs text-muted-foreground mt-0.5">{label}</p>
        </div>
      </CardContent>
    </Card>
  );
}

function ActionCard({
  href,
  icon,
  title,
  description,
}: {
  href: string;
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <Link href={href}>
      <Card className="border border-border hover:border-primary hover:shadow-sm transition-all cursor-pointer h-full">
        <CardContent className="p-4 flex items-start gap-3">
          <div className="h-9 w-9 rounded-lg bg-primary/10 text-primary flex items-center justify-center shrink-0">
            {icon}
          </div>
          <div className="flex-1">
            <p className="text-sm font-medium text-foreground">{title}</p>
            <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
          </div>
          <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
        </CardContent>
      </Card>
    </Link>
  );
}

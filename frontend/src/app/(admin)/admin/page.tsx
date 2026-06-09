"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type AdminDashboardStats, type AdminUserAnalytics, ApiError } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
} from "recharts";
import { format, parseISO } from "date-fns";
import { Users, CalendarDays, TrendingUp, Clock } from "lucide-react";

function StatCard({
  icon: Icon,
  label,
  value,
  color = "text-primary",
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  color?: string;
}) {
  return (
    <Card>
      <CardContent className="p-5 flex items-start gap-4">
        <div className={`mt-0.5 shrink-0 ${color}`}>
          <Icon className="h-5 w-5" />
        </div>
        <div>
          <p className="text-2xl font-bold text-foreground">{value}</p>
          <p className="text-xs text-muted-foreground mt-0.5">{label}</p>
        </div>
      </CardContent>
    </Card>
  );
}

function fmtDay(iso: string) {
  try { return format(parseISO(iso), "d MMM"); } catch { return iso; }
}

function fmtKES(n: number) {
  return `KES ${n.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
}

export default function AdminDashboardPage() {
  const { token, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [stats, setStats] = useState<AdminDashboardStats | null>(null);
  const [analytics, setAnalytics] = useState<AdminUserAnalytics | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    Promise.all([
      api.admin.dashboard(token),
      api.admin.userAnalytics(token),
    ])
      .then(([s, a]) => { setStats(s); setAnalytics(a); })
      .catch((e) => {
        if (e instanceof ApiError && e.status === 403) router.replace("/dashboard");
        else router.replace("/login");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-5 max-w-5xl">
        <Skeleton className="h-8 w-40" />
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => <Skeleton key={i} className="h-24 rounded-xl" />)}
        </div>
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  const u = stats?.users;
  const b = stats?.bookings;

  const signupData = (analytics?.signups_last_7_days ?? [])
    .slice()
    .sort((a, b) => (a.day ?? "") < (b.day ?? "") ? -1 : 1)
    .map((r) => ({ ...r, day: fmtDay(String(r.day)) }));

  return (
    <div className="space-y-6 max-w-5xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Platform Dashboard</h1>
        <p className="text-sm text-muted-foreground mt-1">Overview of MtaaLink activity</p>
      </div>

      {/* User stats */}
      <section className="space-y-3">
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">Users</h2>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <StatCard icon={Users} label="Total users" value={u?.total ?? 0} />
          <StatCard icon={Users} label="Clients" value={u?.clients ?? 0} color="text-blue-500" />
          <StatCard icon={Users} label="Providers" value={u?.providers ?? 0} color="text-green-600" />
          <StatCard icon={Users} label="Businesses" value={u?.businesses ?? 0} color="text-purple-500" />
        </div>
      </section>

      {/* Booking stats */}
      <section className="space-y-3">
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">Bookings</h2>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <StatCard icon={CalendarDays} label="Total bookings" value={b?.total ?? 0} />
          <StatCard icon={Clock} label="Pending" value={b?.pending ?? 0} color="text-yellow-500" />
          <StatCard icon={CalendarDays} label="Confirmed" value={b?.confirmed ?? 0} color="text-green-600" />
          <StatCard icon={CalendarDays} label="Completed" value={b?.completed ?? 0} color="text-primary" />
        </div>
      </section>

      {/* Revenue + Payouts */}
      <section className="space-y-3">
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">Finances</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <StatCard icon={TrendingUp} label="Total revenue collected" value={fmtKES(stats?.revenue.total_collected ?? 0)} color="text-green-600" />
          <StatCard icon={Clock} label="Pending payout amount" value={fmtKES(Number(stats?.payouts.pending_amount ?? 0))} color="text-yellow-500" />
        </div>
      </section>

      {/* New signups chart */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">New signups — last 7 days</CardTitle>
        </CardHeader>
        <CardContent>
          {signupData.length === 0 ? (
            <p className="text-sm text-muted-foreground py-8 text-center">No signups in this period.</p>
          ) : (
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={signupData} margin={{ top: 4, right: 8, left: -20, bottom: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="day" tick={{ fontSize: 11 }} />
                <YAxis allowDecimals={false} tick={{ fontSize: 11 }} />
                <Tooltip formatter={(v) => [v, "Signups"]} contentStyle={{ fontSize: 12 }} />
                <Bar dataKey="count" fill="hsl(var(--primary))" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          )}
        </CardContent>
      </Card>

      {/* Role breakdown */}
      {u && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base">User role breakdown</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={160}>
              <BarChart
                data={[
                  { role: "Clients", count: u.clients },
                  { role: "Providers", count: u.providers },
                  { role: "Businesses", count: u.businesses },
                ]}
                margin={{ top: 4, right: 8, left: -20, bottom: 0 }}
              >
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="role" tick={{ fontSize: 12 }} />
                <YAxis allowDecimals={false} tick={{ fontSize: 11 }} />
                <Tooltip formatter={(v) => [v, "Users"]} contentStyle={{ fontSize: 12 }} />
                <Bar dataKey="count" fill="hsl(var(--primary))" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

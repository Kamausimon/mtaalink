"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type AnalyticsData } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  AreaChart, Area, BarChart, Bar, XAxis, YAxis, CartesianGrid,
  Tooltip, ResponsiveContainer,
} from "recharts";
import { format, parseISO } from "date-fns";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import {
  CalendarDays, TrendingUp, Star, Users, CheckCircle2, XCircle, Clock,
} from "lucide-react";

const PERIODS = [
  { label: "7 days", value: 7 },
  { label: "30 days", value: 30 },
  { label: "90 days", value: 90 },
  { label: "1 year", value: 365 },
];

function StatCard({
  icon: Icon, label, value, sub, color = "text-primary",
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  sub?: string;
  color?: string;
}) {
  return (
    <Card className="border border-border">
      <CardContent className="p-4 flex items-start gap-3">
        <div className={`mt-0.5 shrink-0 ${color}`}>
          <Icon className="h-5 w-5" />
        </div>
        <div>
          <p className="text-xl font-bold text-foreground">{value}</p>
          <p className="text-xs text-muted-foreground">{label}</p>
          {sub && <p className="text-xs text-muted-foreground/70 mt-0.5">{sub}</p>}
        </div>
      </CardContent>
    </Card>
  );
}

function fmtDate(iso: string) {
  try { return format(parseISO(iso), "d MMM"); } catch { return iso; }
}

function fmtKES(n: number) {
  return `KES ${n.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
}

export default function AnalyticsPage() {
  const { token, user, isAuthenticated, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [data, setData] = useState<AnalyticsData | null>(null);
  const [days, setDays] = useState(30);
  const [loading, setLoading] = useState(true);
  const [targetId, setTargetId] = useState<number | null>(null);
  const [targetType, setTargetType] = useState<"provider" | "business">("provider");

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.replace("/login"); return; }
    if (user?.role === "client") { router.replace("/dashboard"); return; }

    api.dashboard.get(token!).then((d) => {
      const type = d.role === "business" ? "business" : "provider";
      const id = type === "business" ? d.business_id : d.provider_id;
      if (!id) { router.replace("/dashboard"); return; }
      setTargetType(type);
      setTargetId(id);
    }).catch(() => router.replace("/dashboard"));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated]);

  useEffect(() => {
    if (!targetId) return;
    setLoading(true);
    api.analytics.get(targetType, targetId, days, token!)
      .then(setData)
      .catch(() => toast.error("Could not load analytics"))
      .finally(() => setLoading(false));
  }, [targetId, targetType, days, token]);

  if (!_hasHydrated || (loading && !data)) {
    return (
      <div className="mx-auto max-w-5xl px-4 sm:px-6 py-8 space-y-5">
        <Skeleton className="h-9 w-48" />
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          {Array.from({ length: 8 }).map((_, i) => <Skeleton key={i} className="h-24 rounded-xl" />)}
        </div>
        <Skeleton className="h-64 rounded-xl" />
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  const ov = data?.overview;

  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 py-8 space-y-6">
      {/* Header + period picker */}
      <div className="flex items-center justify-between flex-wrap gap-3">
        <div>
          <h1 className="text-2xl font-bold text-foreground">Analytics</h1>
          <p className="text-sm text-muted-foreground mt-0.5">Performance overview for your {targetType}</p>
        </div>
        <div className="flex gap-1 bg-muted rounded-lg p-1">
          {PERIODS.map((p) => (
            <button
              key={p.value}
              onClick={() => setDays(p.value)}
              className={cn(
                "px-3 py-1.5 text-sm rounded-md transition-colors font-medium",
                days === p.value
                  ? "bg-white text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>

      {/* Overview stats */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        <StatCard icon={CalendarDays} label="Total bookings" value={ov?.total_bookings ?? 0} />
        <StatCard icon={TrendingUp} label="Revenue" value={fmtKES(ov?.total_revenue ?? 0)} color="text-green-600" />
        <StatCard icon={Star} label="Avg rating" value={ov?.average_rating ? ov.average_rating.toFixed(1) : "—"} sub={`${ov?.review_count ?? 0} reviews`} color="text-yellow-500" />
        <StatCard
          icon={Users}
          label="Repeat clients"
          value={`${Math.round((data?.repeat_clients.repeat_rate ?? 0) * 100)}%`}
          sub={`${data?.repeat_clients.repeat_clients ?? 0} of ${data?.repeat_clients.total_clients ?? 0}`}
          color="text-blue-500"
        />
      </div>

      {/* Booking status breakdown */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {[
          { label: "Confirmed", value: ov?.confirmed ?? 0, icon: CheckCircle2, color: "text-green-600" },
          { label: "Completed", value: ov?.completed ?? 0, icon: CheckCircle2, color: "text-primary" },
          { label: "Pending", value: ov?.pending ?? 0, icon: Clock, color: "text-yellow-500" },
          { label: "Cancelled", value: ov?.cancelled ?? 0, icon: XCircle, color: "text-red-500" },
        ].map((s) => (
          <div key={s.label} className="flex items-center gap-3 bg-muted/40 rounded-lg px-4 py-3">
            <s.icon className={`h-4 w-4 shrink-0 ${s.color}`} />
            <div>
              <p className="text-lg font-bold text-foreground">{s.value}</p>
              <p className="text-xs text-muted-foreground">{s.label}</p>
            </div>
          </div>
        ))}
      </div>

      {/* Bookings over time */}
      <Card className="border border-border">
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Bookings over time</CardTitle>
        </CardHeader>
        <CardContent>
          {(data?.bookings_over_time.length ?? 0) === 0 ? (
            <p className="text-sm text-muted-foreground py-8 text-center">No bookings in this period.</p>
          ) : (
            <ResponsiveContainer width="100%" height={220}>
              <AreaChart data={data!.bookings_over_time} margin={{ top: 4, right: 8, left: -20, bottom: 0 }}>
                <defs>
                  <linearGradient id="bookingGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.2} />
                    <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="date" tickFormatter={fmtDate} tick={{ fontSize: 11 }} />
                <YAxis allowDecimals={false} tick={{ fontSize: 11 }} />
                <Tooltip
                  formatter={(v) => [v, "Bookings"]}
                  labelFormatter={(l) => fmtDate(String(l))}
                  contentStyle={{ fontSize: 12 }}
                />
                <Area
                  type="monotone" dataKey="count" stroke="hsl(var(--primary))"
                  strokeWidth={2} fill="url(#bookingGrad)"
                />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </CardContent>
      </Card>

      {/* Revenue over time */}
      <Card className="border border-border">
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Revenue over time</CardTitle>
        </CardHeader>
        <CardContent>
          {(data?.revenue_over_time.length ?? 0) === 0 ? (
            <p className="text-sm text-muted-foreground py-8 text-center">No revenue in this period.</p>
          ) : (
            <ResponsiveContainer width="100%" height={220}>
              <AreaChart data={data!.revenue_over_time} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
                <defs>
                  <linearGradient id="revenueGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#16a34a" stopOpacity={0.2} />
                    <stop offset="95%" stopColor="#16a34a" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="date" tickFormatter={fmtDate} tick={{ fontSize: 11 }} />
                <YAxis tickFormatter={(v) => `${(v / 1000).toFixed(0)}k`} tick={{ fontSize: 11 }} />
                <Tooltip
                  formatter={(v) => [fmtKES(Number(v)), "Revenue"]}
                  labelFormatter={(l) => fmtDate(String(l))}
                  contentStyle={{ fontSize: 12 }}
                />
                <Area
                  type="monotone" dataKey="amount" stroke="#16a34a"
                  strokeWidth={2} fill="url(#revenueGrad)"
                />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </CardContent>
      </Card>

      {/* Top services */}
      <Card className="border border-border">
        <CardHeader className="pb-2">
          <CardTitle className="text-base">Top services</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {(data?.top_services.length ?? 0) === 0 ? (
            <p className="text-sm text-muted-foreground px-5 py-6">No bookings yet.</p>
          ) : (
            <>
              <div className="hidden sm:block px-5 pb-4">
                <ResponsiveContainer width="100%" height={180}>
                  <BarChart
                    data={data!.top_services.slice(0, 6)}
                    margin={{ top: 4, right: 8, left: -20, bottom: 40 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                    <XAxis
                      dataKey="service_name"
                      tick={{ fontSize: 10 }}
                      angle={-25}
                      textAnchor="end"
                    />
                    <YAxis allowDecimals={false} tick={{ fontSize: 11 }} />
                    <Tooltip
                      formatter={(v) => [v, "Bookings"]}
                      contentStyle={{ fontSize: 12 }}
                    />
                    <Bar dataKey="booking_count" fill="hsl(var(--primary))" radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
              <Separator />
              {data!.top_services.map((s, i) => (
                <div key={i}>
                  {i > 0 && <Separator />}
                  <div className="flex items-center justify-between px-5 py-3">
                    <div className="flex items-center gap-3">
                      <span className="text-xs font-bold text-muted-foreground w-5 text-right">{i + 1}</span>
                      <p className="text-sm font-medium text-foreground">{s.service_name ?? "Custom"}</p>
                    </div>
                    <div className="flex items-center gap-6 text-sm">
                      <span className="text-muted-foreground">{s.booking_count} bookings</span>
                      <span className="font-semibold text-green-600">{fmtKES(s.revenue)}</span>
                    </div>
                  </div>
                </div>
              ))}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

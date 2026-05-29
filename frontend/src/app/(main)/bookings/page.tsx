"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Booking } from "@/lib/api";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CalendarCheck, MapPin, Clock } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";

const STATUS_COLORS: Record<string, string> = {
  pending: "bg-amber-100 text-amber-700 border-amber-200",
  confirmed: "bg-green-100 text-green-700 border-green-200",
  completed: "bg-blue-100 text-blue-700 border-blue-200",
  cancelled: "bg-red-100 text-red-700 border-red-200",
};

export default function BookingsPage() {
  const { token, isAuthenticated } = useAuthStore();
  const router = useRouter();
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<string>("all");

  useEffect(() => {
    if (!isAuthenticated) {
      router.push("/login");
      return;
    }
    loadBookings();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated, tab]);

  async function loadBookings() {
    setLoading(true);
    try {
      const res = await api.bookings.myBookings(token!, {
        status: tab === "all" ? undefined : tab,
      });
      setBookings(res.bookings);
    } catch {
      toast.error("Could not load bookings");
    } finally {
      setLoading(false);
    }
  }

  async function cancelBooking(id: number) {
    const reason = window.prompt("Reason for cancellation (optional):");
    if (reason === null) return; // user hit Cancel
    try {
      await api.bookings.delete(id, token!);
      toast.success("Booking cancelled");
      loadBookings();
    } catch {
      toast.error("Could not cancel booking");
    }
  }

  const TABS = ["all", "pending", "confirmed", "completed", "cancelled"];

  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 py-8 space-y-6">
      <h1 className="text-2xl font-bold text-foreground">My Bookings</h1>

      <Tabs value={tab} onValueChange={setTab}>
        <TabsList className="bg-muted/50 overflow-x-auto flex-nowrap w-full justify-start">
          {TABS.map((t) => (
            <TabsTrigger key={t} value={t} className="capitalize text-xs shrink-0">
              {t}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-28 rounded-xl" />
          ))}
        </div>
      ) : bookings.length === 0 ? (
        <div className="text-center py-16">
          <CalendarCheck className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
          <p className="font-medium text-foreground mb-1">No bookings found</p>
          <p className="text-sm text-muted-foreground mb-4">
            {tab === "all"
              ? "You haven't made any bookings yet."
              : `No ${tab} bookings.`}
          </p>
          <Button onClick={() => router.push("/search")}>Find a provider</Button>
        </div>
      ) : (
        <div className="space-y-3">
          {bookings.map((b) => (
            <Card key={b.id} className="border border-border">
              <CardContent className="p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0 space-y-1.5">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-medium text-foreground">
                        Booking #{b.id}
                      </span>
                      <span
                        className={`text-xs px-2 py-0.5 rounded-full border font-medium capitalize ${
                          STATUS_COLORS[b.status] ?? "bg-muted text-muted-foreground border-border"
                        }`}
                      >
                        {b.status}
                      </span>
                    </div>

                    <p className="text-sm text-muted-foreground truncate">
                      {b.service_description ?? "Service booking"}
                    </p>

                    <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {format(new Date(b.scheduled_time), "d MMM yyyy, h:mm a")}
                      </span>
                      {b.client_address && (
                        <span className="flex items-center gap-1">
                          <MapPin className="h-3 w-3" />
                          {b.client_address}
                        </span>
                      )}
                    </div>

                    {b.cancel_reason && (
                      <p className="text-xs text-destructive">
                        Cancelled: {b.cancel_reason}
                      </p>
                    )}
                  </div>

                  {b.status === "pending" && (
                    <Button
                      size="sm"
                      variant="outline"
                      className="shrink-0 text-destructive hover:text-destructive"
                      onClick={() => cancelBooking(b.id)}
                    >
                      Cancel
                    </Button>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

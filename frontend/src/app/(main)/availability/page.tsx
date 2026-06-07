"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Save } from "lucide-react";
import { toast } from "sonner";

const DAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

type DaySchedule = {
  day: string;
  is_available: boolean;
  start_time: string;
  end_time: string;
};

function defaultSchedule(): DaySchedule[] {
  return DAYS.map((day) => ({
    day,
    is_available: !["Saturday", "Sunday"].includes(day),
    start_time: "08:00",
    end_time: "17:00",
  }));
}

export default function AvailabilityPage() {
  const { token, isAuthenticated, user, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [schedule, setSchedule] = useState<DaySchedule[]>(defaultSchedule());
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [providerId, setProviderId] = useState<number | null>(null);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) {
      router.push("/login");
      return;
    }
    if (user?.role !== "provider") {
      router.push("/dashboard");
      return;
    }

    api.dashboard.get(token!).then(async (dash) => {
      if (!dash.provider_id) {
        toast.error("Provider profile not found. Please complete onboarding first.");
        router.push("/dashboard");
        return;
      }
      setProviderId(dash.provider_id);
      try {
        const res = await api.providers.availability.get(dash.provider_id);
        if (res.schedule && res.schedule.length > 0) {
          const merged = DAYS.map((day) => {
            const saved = res.schedule.find((s) => s.day === day);
            return saved
              ? {
                  day,
                  is_available: saved.is_available,
                  start_time: saved.start_time?.slice(0, 5) ?? "08:00",
                  end_time: saved.end_time?.slice(0, 5) ?? "17:00",
                }
              : { day, is_available: false, start_time: "08:00", end_time: "17:00" };
          });
          setSchedule(merged);
        }
      } catch {
        // no saved schedule yet — use defaults
      } finally {
        setLoading(false);
      }
    }).catch(() => {
      toast.error("Could not load profile");
      setLoading(false);
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated]);

  function toggleDay(day: string) {
    setSchedule((prev) =>
      prev.map((d) => (d.day === day ? { ...d, is_available: !d.is_available } : d))
    );
  }

  function setTime(day: string, field: "start_time" | "end_time", value: string) {
    setSchedule((prev) =>
      prev.map((d) => (d.day === day ? { ...d, [field]: value } : d))
    );
  }

  async function save() {
    if (!providerId) return;
    setSaving(true);
    try {
      await api.providers.availability.set(providerId, schedule, token!);
      toast.success("Availability saved");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Could not save availability");
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-4">
        <Skeleton className="h-8 w-48" />
        {Array.from({ length: 7 }).map((_, i) => (
          <Skeleton key={i} className="h-16 rounded-xl" />
        ))}
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-foreground">Availability</h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            Set which days and hours clients can book you.
          </p>
        </div>
        <Button onClick={save} disabled={saving} className="gap-2">
          <Save className="h-4 w-4" />
          {saving ? "Saving…" : "Save"}
        </Button>
      </div>

      <Card className="border border-border">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
            Weekly schedule
          </CardTitle>
        </CardHeader>
        <CardContent className="divide-y divide-border">
          {schedule.map((day) => (
            <div key={day.day} className="flex items-center gap-4 py-3">
              <div className="flex items-center gap-3 w-32 shrink-0">
                <Switch
                  id={`toggle-${day.day}`}
                  checked={day.is_available}
                  onCheckedChange={() => toggleDay(day.day)}
                />
                <Label
                  htmlFor={`toggle-${day.day}`}
                  className={`text-sm font-medium cursor-pointer select-none ${
                    day.is_available ? "text-foreground" : "text-muted-foreground"
                  }`}
                >
                  {day.day.slice(0, 3)}
                </Label>
              </div>

              {day.is_available ? (
                <div className="flex items-center gap-2 flex-1">
                  <Input
                    type="time"
                    value={day.start_time}
                    onChange={(e) => setTime(day.day, "start_time", e.target.value)}
                    className="w-32 text-sm"
                  />
                  <span className="text-xs text-muted-foreground">to</span>
                  <Input
                    type="time"
                    value={day.end_time}
                    onChange={(e) => setTime(day.day, "end_time", e.target.value)}
                    className="w-32 text-sm"
                  />
                </div>
              ) : (
                <span className="text-sm text-muted-foreground">Not available</span>
              )}
            </div>
          ))}
        </CardContent>
      </Card>

      <div className="flex justify-end">
        <Button onClick={save} disabled={saving} className="gap-2">
          <Save className="h-4 w-4" />
          {saving ? "Saving…" : "Save schedule"}
        </Button>
      </div>
    </div>
  );
}

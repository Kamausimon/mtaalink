"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { useAuthStore } from "@/store/auth";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";

const providerSchema = z.object({
  service_name: z.string().min(3).optional(),
  service_description: z.string().min(10).optional(),
  location: z.string().optional(),
  phone_number: z.string().optional(),
  email: z.string().email().optional(),
  website: z.string().url().optional().or(z.literal("")),
  whatsapp: z.string().optional(),
});
type ProviderForm = z.infer<typeof providerSchema>;

export default function ProfilePage() {
  const { token, user, isAuthenticated } = useAuthStore();
  const router = useRouter();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!isAuthenticated) { router.push("/login"); return; }
    setLoading(false);
  }, [isAuthenticated, router]);

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ProviderForm>({ resolver: zodResolver(providerSchema) });

  async function onSubmit(data: ProviderForm) {
    if (!user) return;
    setSaving(true);
    try {
      if (user.role === "provider") {
        await api.providers.updateProfile(
          {
            service_name: data.service_name || undefined,
            service_description: data.service_description || undefined,
            location: data.location || undefined,
            phone_number: data.phone_number || undefined,
            email: data.email || undefined,
            website: data.website || undefined,
            whatsapp: data.whatsapp || undefined,
          },
          token!,
        );
      }
      toast.success("Profile updated");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Update failed");
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div className="mx-auto max-w-lg px-4 sm:px-6 py-8 space-y-4">
        <Skeleton className="h-24 rounded-xl" />
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  const initials = user?.username?.slice(0, 2).toUpperCase() ?? "??";

  return (
    <div className="mx-auto max-w-lg px-4 sm:px-6 py-8 space-y-6">
      <h1 className="text-2xl font-bold text-foreground">Profile Settings</h1>

      {/* Account info */}
      <Card className="border border-border">
        <CardContent className="p-5 flex items-center gap-4">
          <Avatar className="h-14 w-14">
            <AvatarFallback className="bg-primary/10 text-primary font-bold text-lg">
              {initials}
            </AvatarFallback>
          </Avatar>
          <div>
            <p className="font-semibold text-foreground">{user?.username}</p>
            <p className="text-sm text-muted-foreground">{user?.email}</p>
            <Badge variant="secondary" className="mt-1 capitalize text-xs">
              {user?.role}
            </Badge>
          </div>
        </CardContent>
      </Card>

      {/* Provider profile update */}
      {user?.role === "provider" && (
        <Card className="border border-border">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">Provider profile</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
              <div className="space-y-1.5">
                <Label>Service name</Label>
                <Input placeholder="Your trading name" {...register("service_name")} />
              </div>
              <div className="space-y-1.5">
                <Label>About your services</Label>
                <Textarea rows={3} placeholder="What you do…" {...register("service_description")} />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label>Location</Label>
                  <Input placeholder="e.g. Westlands" {...register("location")} />
                </div>
                <div className="space-y-1.5">
                  <Label>Phone</Label>
                  <Input type="tel" placeholder="07XX XXX XXX" {...register("phone_number")} />
                </div>
              </div>
              <div className="space-y-1.5">
                <Label>Email</Label>
                <Input type="email" {...register("email")} />
                {errors.email && <p className="text-xs text-destructive">{errors.email.message}</p>}
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label>Website</Label>
                  <Input placeholder="https://…" {...register("website")} />
                </div>
                <div className="space-y-1.5">
                  <Label>WhatsApp</Label>
                  <Input placeholder="07XX XXX XXX" {...register("whatsapp")} />
                </div>
              </div>
              <Button type="submit" disabled={saving} className="w-full">
                {saving ? "Saving…" : "Save changes"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      {/* Danger zone */}
      <Card className="border border-border">
        <CardHeader className="pb-3">
          <CardTitle className="text-base text-destructive">Account</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Separator />
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-foreground">Log out</p>
              <p className="text-xs text-muted-foreground">Sign out of your account</p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                useAuthStore.getState().clearAuth();
                router.push("/");
              }}
            >
              Log out
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

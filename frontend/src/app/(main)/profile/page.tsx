"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { Camera, Loader2 } from "lucide-react";
import { useAuthStore } from "@/store/auth";
import { api } from "@/lib/api";
import { uploadToCloudinary } from "@/lib/cloudinary";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";

// Allow empty string (field untouched) or a string meeting the minimum length.
// Empty strings are converted to undefined in onSubmit so the backend skips them.
const optStr = (min?: number, label?: string) =>
  z
    .string()
    .refine((v) => v === "" || !min || v.length >= min, {
      message: `${label ?? "Field"} must be at least ${min} characters`,
    })
    .optional();

const providerSchema = z.object({
  service_name: optStr(3, "Service name"),
  service_description: optStr(10, "Description"),
  location: z.string().optional(),
  phone_number: z.string().optional(),
  website: z
    .string()
    .refine((v) => v === "" || z.string().url().safeParse(v).success, {
      message: "Must be a valid URL (include https://)",
    })
    .optional(),
  whatsapp: z.string().optional(),
});
type ProviderForm = z.infer<typeof providerSchema>;

const businessSchema = z.object({
  description: optStr(10, "Description"),
  phone_number: optStr(10, "Phone number"),
  website: z
    .string()
    .refine((v) => v === "" || z.string().url().safeParse(v).success, {
      message: "Must be a valid URL (include https://)",
    })
    .optional(),
  whatsapp: z.string().optional(),
});
type BusinessForm = z.infer<typeof businessSchema>;

export default function ProfilePage() {
  const { token, user, isAuthenticated, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [contactEmail, setContactEmail] = useState<string | undefined>(undefined);
  const [photoUrl, setPhotoUrl] = useState<string | null>(null);
  const [photoUploading, setPhotoUploading] = useState(false);
  const [businessId, setBusinessId] = useState<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const providerForm = useForm<ProviderForm>({ resolver: zodResolver(providerSchema) });
  const businessForm = useForm<BusinessForm>({ resolver: zodResolver(businessSchema) });

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) {
      router.push("/login");
      return;
    }

    if (user?.role === "provider") {
      api.providers.getMyData(token!).then((res) => {
        const p = res.provider_data;
        providerForm.reset({
          service_name: p.service_name ?? "",
          service_description: p.service_description ?? "",
          location: p.location ?? "",
          phone_number: p.phone_number ?? "",
          website: p.website ?? "",
          whatsapp: p.whatsapp ?? "",
        });
        setContactEmail(p.email);
        setPhotoUrl(p.profile_photo ?? null);
      }).catch((err: unknown) => {
        if (err instanceof Error && !err.message.includes("not found")) {
          toast.error("Could not load profile data");
        }
      }).finally(() => setLoading(false));
    } else if (user?.role === "business") {
      api.dashboard.get(token!).then((d) => {
        if (!d.business_id) { setLoading(false); return; }
        setBusinessId(d.business_id);
        return api.businesses.getById(d.business_id).then(({ business: b }) => {
          businessForm.reset({
            description: b.description ?? "",
            phone_number: b.phone_number ?? "",
            website: b.website ?? "",
            whatsapp: b.whatsapp ?? "",
          });
          setContactEmail(b.email);
          setPhotoUrl(b.profile_photo ?? null);
        });
      }).catch(() => {
        toast.error("Could not load business data");
      }).finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated, router, token, user?.role]);

  async function handlePhotoChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file || !user) return;
    setPhotoUploading(true);
    try {
      const url = await uploadToCloudinary(file);
      if (user.role === "provider") {
        await api.providers.updateProfile({ profile_photo: url }, token!);
      } else if (user.role === "business") {
        await api.businesses.updateProfile({ profile_photo: url }, token!);
      }
      setPhotoUrl(url);
      toast.success("Profile photo updated");
    } catch {
      toast.error("Photo upload failed. Try again.");
    } finally {
      setPhotoUploading(false);
    }
  }

  async function onSubmitProvider(data: ProviderForm) {
    setSaving(true);
    try {
      await api.providers.updateProfile(
        {
          service_name: data.service_name || undefined,
          service_description: data.service_description || undefined,
          location: data.location || undefined,
          phone_number: data.phone_number || undefined,
          website: data.website || undefined,
          whatsapp: data.whatsapp || undefined,
        },
        token!,
      );
      toast.success("Profile updated");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Update failed");
    } finally {
      setSaving(false);
    }
  }

  async function onSubmitBusiness(data: BusinessForm) {
    setSaving(true);
    try {
      await api.businesses.updateProfile(
        {
          description: data.description || undefined,
          phone_number: data.phone_number || undefined,
          website: data.website || undefined,
          whatsapp: data.whatsapp || undefined,
        },
        token!,
      );
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
  const canUploadPhoto = user?.role === "provider" || user?.role === "business";

  return (
    <div className="mx-auto max-w-lg px-4 sm:px-6 py-8 space-y-6">
      <h1 className="text-2xl font-bold text-foreground">Profile Settings</h1>

      {/* Account info */}
      <Card className="border border-border">
        <CardContent className="p-5 flex items-center gap-4">
          <div className="relative shrink-0">
            <Avatar className="h-14 w-14">
              <AvatarImage src={photoUrl ?? undefined} alt={user?.username ?? ""} />
              <AvatarFallback className="bg-primary/10 text-primary font-bold text-lg">
                {initials}
              </AvatarFallback>
            </Avatar>
            {canUploadPhoto && (
              <>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  className="hidden"
                  onChange={handlePhotoChange}
                />
                <button
                  type="button"
                  onClick={() => fileInputRef.current?.click()}
                  disabled={photoUploading}
                  title="Change profile photo"
                  className="absolute -bottom-1 -right-1 flex h-6 w-6 items-center justify-center rounded-full bg-primary text-white border-2 border-white hover:bg-primary/90 transition-colors"
                >
                  {photoUploading ? (
                    <Loader2 className="h-3 w-3 animate-spin" />
                  ) : (
                    <Camera className="h-3 w-3" />
                  )}
                </button>
              </>
            )}
          </div>
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
            <form onSubmit={providerForm.handleSubmit(onSubmitProvider)} className="space-y-4">
              <div className="space-y-1.5">
                <Label>Service name</Label>
                <Input placeholder="Your trading name" {...providerForm.register("service_name")} />
                {providerForm.formState.errors.service_name && (
                  <p className="text-xs text-destructive">{providerForm.formState.errors.service_name.message}</p>
                )}
              </div>
              <div className="space-y-1.5">
                <Label>About your services</Label>
                <Textarea rows={3} placeholder="What you do…" {...providerForm.register("service_description")} />
                {providerForm.formState.errors.service_description && (
                  <p className="text-xs text-destructive">{providerForm.formState.errors.service_description.message}</p>
                )}
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label>Location</Label>
                  <Input placeholder="e.g. Westlands" {...providerForm.register("location")} />
                </div>
                <div className="space-y-1.5">
                  <Label>Phone</Label>
                  <Input type="tel" placeholder="07XX XXX XXX" {...providerForm.register("phone_number")} />
                </div>
              </div>
              <div className="space-y-1.5">
                <Label>Contact email</Label>
                <Input type="email" value={contactEmail ?? ""} disabled className="bg-muted text-muted-foreground" />
                <p className="text-xs text-muted-foreground">
                  Your email can&apos;t be changed. Contact support if you need to update it.
                </p>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label>Website</Label>
                  <Input placeholder="https://…" {...providerForm.register("website")} />
                  {providerForm.formState.errors.website && (
                    <p className="text-xs text-destructive">{providerForm.formState.errors.website.message}</p>
                  )}
                </div>
                <div className="space-y-1.5">
                  <Label>WhatsApp</Label>
                  <Input placeholder="07XX XXX XXX" {...providerForm.register("whatsapp")} />
                </div>
              </div>
              <Button type="submit" disabled={saving} className="w-full">
                {saving ? "Saving…" : "Save changes"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      {/* Business profile update */}
      {user?.role === "business" && businessId && (
        <Card className="border border-border">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">Business profile</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={businessForm.handleSubmit(onSubmitBusiness)} className="space-y-4">
              <div className="space-y-1.5">
                <Label>About your business</Label>
                <Textarea rows={3} placeholder="What your business does…" {...businessForm.register("description")} />
                {businessForm.formState.errors.description && (
                  <p className="text-xs text-destructive">{businessForm.formState.errors.description.message}</p>
                )}
              </div>
              <div className="space-y-1.5">
                <Label>Phone</Label>
                <Input type="tel" placeholder="07XX XXX XXX" {...businessForm.register("phone_number")} />
                {businessForm.formState.errors.phone_number && (
                  <p className="text-xs text-destructive">{businessForm.formState.errors.phone_number.message}</p>
                )}
              </div>
              <div className="space-y-1.5">
                <Label>Contact email</Label>
                <Input type="email" value={contactEmail ?? ""} disabled className="bg-muted text-muted-foreground" />
                <p className="text-xs text-muted-foreground">
                  Your email can&apos;t be changed. Contact support if you need to update it.
                </p>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label>Website</Label>
                  <Input placeholder="https://…" {...businessForm.register("website")} />
                  {businessForm.formState.errors.website && (
                    <p className="text-xs text-destructive">{businessForm.formState.errors.website.message}</p>
                  )}
                </div>
                <div className="space-y-1.5">
                  <Label>WhatsApp</Label>
                  <Input placeholder="07XX XXX XXX" {...businessForm.register("whatsapp")} />
                </div>
              </div>
              <Button type="submit" disabled={saving} className="w-full">
                {saving ? "Saving…" : "Save changes"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      {/* Account actions */}
      <Card className="border border-border">
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Account</CardTitle>
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

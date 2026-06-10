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
import { LocationCombobox } from "@/components/LocationCombobox";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";

const schema = z.object({
  service_name: z.string().min(3, "Service name must be at least 3 characters"),
  service_description: z.string().min(10, "Tell clients more about what you do"),
  category: z.string().optional(),
  location: z.string().optional(),
  phone_number: z.string().min(10, "Enter a valid phone number").optional().or(z.literal("")),
  email: z.string().email("Enter a valid email"),
  website: z.string().url("Enter a valid URL").optional().or(z.literal("")),
  whatsapp: z.string().optional(),
});
type FormData = z.infer<typeof schema>;

const CATEGORIES = [
  "Plumbing", "Electrical", "Cleaning", "Tutoring", "Painting",
  "Security", "Catering", "Beauty", "Construction", "Photography",
  "Mechanics", "IT & Tech", "Gardening", "Other",
];

export default function ProviderOnboardPage() {
  const { token, user, isAuthenticated, updateUser } = useAuthStore();
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [photoUrl, setPhotoUrl] = useState<string | null>(null);
  const [photoUploading, setPhotoUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  async function handlePhotoChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setPhotoUploading(true);
    try {
      const url = await uploadToCloudinary(file);
      setPhotoUrl(url);
    } catch {
      toast.error("Photo upload failed. Try again.");
    } finally {
      setPhotoUploading(false);
    }
  }

  useEffect(() => {
    if (!isAuthenticated || user?.role !== "provider") {
      router.push("/dashboard");
    }
  }, [isAuthenticated, user, router]);

  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<FormData>({
    resolver: zodResolver(schema),
    defaultValues: { email: user?.email ?? "" },
  });

  const selectedCategory = watch("category");

  async function onSubmit(data: FormData) {
    setLoading(true);
    try {
      await api.providers.onboard(
        {
          service_name: data.service_name,
          service_description: data.service_description,
          category: data.category || undefined,
          location: data.location || undefined,
          phone_number: data.phone_number || undefined,
          email: data.email,
          website: data.website || undefined,
          whatsapp: data.whatsapp || undefined,
          profile_photo: photoUrl || undefined,
        },
        token!,
      );
      if (user) updateUser({ ...user, onboarding_completed: true });
      toast.success("Profile set up! You can now start receiving bookings.");
      router.push("/dashboard");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Setup failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="mx-auto max-w-lg px-4 sm:px-6 py-10">
      <Card className="border border-border shadow-none">
        <CardHeader>
          <CardTitle className="text-2xl font-bold">Set up your provider profile</CardTitle>
          <CardDescription>
            Tell clients who you are and what you offer. You can update this anytime.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
            {/* Profile photo */}
            <div className="flex flex-col items-center gap-3">
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
                className="relative w-24 h-24 rounded-full border-2 border-dashed border-border bg-muted hover:border-primary transition-colors overflow-hidden"
                disabled={photoUploading}
              >
                {photoUrl ? (
                  <img src={photoUrl} alt="Profile" className="w-full h-full object-cover" />
                ) : (
                  <div className="flex flex-col items-center justify-center h-full gap-1">
                    {photoUploading ? (
                      <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                    ) : (
                      <>
                        <Camera className="h-5 w-5 text-muted-foreground" />
                        <span className="text-[10px] text-muted-foreground">Add photo</span>
                      </>
                    )}
                  </div>
                )}
              </button>
              <p className="text-xs text-muted-foreground">Profile photo (optional)</p>
            </div>

            <div className="space-y-1.5">
              <Label>Your service / trade name</Label>
              <Input placeholder="e.g. John's Plumbing Services" {...register("service_name")} />
              {errors.service_name && (
                <p className="text-xs text-destructive">{errors.service_name.message}</p>
              )}
            </div>

            <div className="space-y-1.5">
              <Label>About your services</Label>
              <Textarea
                placeholder="Describe what you do, your experience, and your service area…"
                rows={4}
                {...register("service_description")}
              />
              {errors.service_description && (
                <p className="text-xs text-destructive">{errors.service_description.message}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label>Category</Label>
              <div className="flex flex-wrap gap-2">
                {CATEGORIES.map((cat) => (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => setValue("category", cat)}
                    className={`px-3 py-1.5 rounded-full text-xs font-medium border transition-colors ${
                      selectedCategory === cat
                        ? "bg-primary text-white border-primary"
                        : "bg-white text-foreground border-border hover:border-primary"
                    }`}
                  >
                    {cat}
                  </button>
                ))}
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label>Location / area</Label>
                <LocationCombobox
                  value={watch("location") ?? ""}
                  onChange={(val) => setValue("location", val, { shouldValidate: true })}
                  placeholder="e.g. Westlands, Nairobi"
                />
              </div>
              <div className="space-y-1.5">
                <Label>Phone number</Label>
                <Input type="tel" placeholder="07XX XXX XXX" {...register("phone_number")} />
                {errors.phone_number && (
                  <p className="text-xs text-destructive">{errors.phone_number.message}</p>
                )}
              </div>
            </div>

            <div className="space-y-1.5">
              <Label>Contact email</Label>
              <Input type="email" {...register("email")} />
              {errors.email && (
                <p className="text-xs text-destructive">{errors.email.message}</p>
              )}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label>Website <span className="text-muted-foreground">(optional)</span></Label>
                <Input placeholder="https://…" {...register("website")} />
              </div>
              <div className="space-y-1.5">
                <Label>WhatsApp <span className="text-muted-foreground">(optional)</span></Label>
                <Input placeholder="07XX XXX XXX" {...register("whatsapp")} />
              </div>
            </div>

            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? "Saving…" : "Save and go to dashboard"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

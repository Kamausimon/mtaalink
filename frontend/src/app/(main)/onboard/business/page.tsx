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
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";

const schema = z.object({
  business_name: z.string().min(3, "Business name must be at least 3 characters"),
  description: z.string().min(10, "Tell clients about your business"),
  category: z.string().optional(),
  location: z.string().optional(),
  license_number: z.string().min(1, "License number is required"),
  krapin: z.string().min(11, "Enter a valid KRA PIN (11 characters)"),
  phone_number: z.string().min(10, "Enter a valid phone number"),
  email: z.string().email("Enter a valid email"),
  website: z.string().url("Enter a valid URL").optional().or(z.literal("")),
  whatsapp: z.string().optional(),
});
type FormData = z.infer<typeof schema>;

const CATEGORIES = [
  "Cleaning", "Security", "Catering", "Construction", "IT & Tech",
  "Beauty & Wellness", "Healthcare", "Education", "Logistics", "Other",
];

export default function BusinessOnboardPage() {
  const { token, user, isAuthenticated } = useAuthStore();
  const router = useRouter();
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isAuthenticated || user?.role !== "business") {
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
      await api.businesses.onboard(
        {
          business_name: data.business_name,
          description: data.description,
          category: data.category || undefined,
          location: data.location || undefined,
          license_number: data.license_number,
          krapin: data.krapin,
          phone_number: data.phone_number,
          email: data.email,
          website: data.website || undefined,
          whatsapp: data.whatsapp || undefined,
        },
        token!,
      );
      toast.success("Business profile created! You can now receive bookings.");
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
          <CardTitle className="text-2xl font-bold">Set up your business profile</CardTitle>
          <CardDescription>
            You&apos;ll need your business license number and KRA PIN to complete registration.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
            <div className="space-y-1.5">
              <Label>Business name</Label>
              <Input placeholder="e.g. Bright Cleaning Solutions Ltd" {...register("business_name")} />
              {errors.business_name && (
                <p className="text-xs text-destructive">{errors.business_name.message}</p>
              )}
            </div>

            <div className="space-y-1.5">
              <Label>About your business</Label>
              <Textarea
                placeholder="Describe your services, experience, and what makes you different…"
                rows={4}
                {...register("description")}
              />
              {errors.description && (
                <p className="text-xs text-destructive">{errors.description.message}</p>
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
                <Input placeholder="e.g. CBD, Nairobi" {...register("location")} />
              </div>
              <div className="space-y-1.5">
                <Label>Phone number</Label>
                <Input type="tel" placeholder="07XX XXX XXX" {...register("phone_number")} />
                {errors.phone_number && (
                  <p className="text-xs text-destructive">{errors.phone_number.message}</p>
                )}
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label>License number</Label>
                <Input placeholder="BN/XXXX/XXXX" {...register("license_number")} />
                {errors.license_number && (
                  <p className="text-xs text-destructive">{errors.license_number.message}</p>
                )}
              </div>
              <div className="space-y-1.5">
                <Label>KRA PIN</Label>
                <Input placeholder="AXXXXXXXXX" {...register("krapin")} />
                {errors.krapin && (
                  <p className="text-xs text-destructive">{errors.krapin.message}</p>
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

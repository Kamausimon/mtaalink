"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { api, type ProviderProfile, type Service, type Review } from "@/lib/api";
import { useAuthStore } from "@/store/auth";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Star,
  MapPin,
  Phone,
  Globe,
  MessageCircle,
  Heart,
  CheckCircle,
  Clock,
} from "lucide-react";
import { toast } from "sonner";
import { format } from "date-fns";

export default function ProviderProfilePage() {
  const { id } = useParams<{ id: string }>();
  const router = useRouter();
  const { token, isAuthenticated } = useAuthStore();

  const [provider, setProvider] = useState<ProviderProfile | null>(null);
  const [services, setServices] = useState<Service[]>([]);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [loading, setLoading] = useState(true);
  const [bookingOpen, setBookingOpen] = useState(false);
  const [selectedService, setSelectedService] = useState<Service | null>(null);
  const [bookingLoading, setBookingLoading] = useState(false);
  const [bookingForm, setBookingForm] = useState({
    scheduled_time: "",
    service_description: "",
    client_phone: "",
    client_address: "",
  });

  useEffect(() => {
    async function load() {
      try {
        const [profileRes, reviewRes] = await Promise.all([
          api.providers.getById(Number(id)),
          api.reviews.get("provider", Number(id)),
        ]);
        setProvider(profileRes.provider);
        setServices(profileRes.services);
        setReviews(reviewRes.reviews);
      } catch {
        toast.error("Provider not found");
        router.push("/search");
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [id, router]);

  async function handleBook() {
    if (!isAuthenticated) {
      router.push(`/login?next=/providers/${id}`);
      return;
    }
    setBookingOpen(true);
  }

  async function submitBooking() {
    if (!bookingForm.scheduled_time) {
      toast.error("Please pick a date and time");
      return;
    }
    if (!bookingForm.service_description && !selectedService) {
      toast.error("Please describe the work needed");
      return;
    }
    setBookingLoading(true);
    try {
      const res = await api.bookings.create(
        {
          target_type: "provider",
          target_id: Number(id),
          service_id: selectedService?.id,
          service_description:
            bookingForm.service_description || selectedService?.title || "",
          scheduled_time: bookingForm.scheduled_time,
          client_phone: bookingForm.client_phone || undefined,
          client_address: bookingForm.client_address || undefined,
        },
        token!,
      );
      toast.success(`Booking #${res.booking_id} created! The provider will confirm shortly.`);
      setBookingOpen(false);
      router.push("/bookings");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Booking failed");
    } finally {
      setBookingLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="mx-auto max-w-3xl px-4 sm:px-6 py-10 space-y-6">
        <Skeleton className="h-40 rounded-xl" />
        <Skeleton className="h-32 rounded-xl" />
        <Skeleton className="h-48 rounded-xl" />
      </div>
    );
  }

  if (!provider) return null;

  const avgRating = provider.avg_rating;
  const reviewCount = provider.review_count ?? 0;

  return (
    <>
      <div className="mx-auto max-w-3xl px-4 sm:px-6 py-8 space-y-6">
        {/* Header card */}
        <Card className="border border-border overflow-hidden">
          {provider.cover_photo && (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={provider.cover_photo}
              alt="Cover"
              className="w-full h-36 object-cover"
            />
          )}
          <CardContent className="p-5">
            <div className="flex items-start gap-4">
              <Avatar className="h-16 w-16 border-2 border-white shadow-sm -mt-10 relative">
                <AvatarImage src={provider.profile_photo ?? undefined} />
                <AvatarFallback className="bg-primary/10 text-primary font-bold text-lg">
                  {provider.service_name?.slice(0, 2).toUpperCase()}
                </AvatarFallback>
              </Avatar>
              <div className="flex-1 min-w-0">
                <h1 className="text-xl font-bold text-foreground">
                  {provider.service_name}
                </h1>
                <div className="flex flex-wrap items-center gap-3 mt-1">
                  {provider.category && (
                    <Badge variant="secondary">{provider.category}</Badge>
                  )}
                  {avgRating != null && (
                    <div className="flex items-center gap-1 text-sm">
                      <Star className="h-4 w-4 fill-accent text-accent" />
                      <span className="font-medium">{avgRating.toFixed(1)}</span>
                      <span className="text-muted-foreground">
                        ({reviewCount} {reviewCount === 1 ? "review" : "reviews"})
                      </span>
                    </div>
                  )}
                  {provider.location && (
                    <div className="flex items-center gap-1 text-sm text-muted-foreground">
                      <MapPin className="h-3.5 w-3.5" />
                      {provider.location}
                    </div>
                  )}
                </div>
              </div>
            </div>

            {provider.service_description && (
              <p className="mt-4 text-sm text-muted-foreground leading-relaxed">
                {provider.service_description}
              </p>
            )}

            <div className="flex flex-wrap items-center gap-3 mt-5">
              <Button onClick={handleBook} className="gap-2">
                Book this provider
              </Button>
              {provider.phone_number && (
                <a href={`tel:${provider.phone_number}`}>
                  <Button variant="outline" size="icon">
                    <Phone className="h-4 w-4" />
                  </Button>
                </a>
              )}
              {provider.website && (
                <a href={provider.website} target="_blank" rel="noopener noreferrer">
                  <Button variant="outline" size="icon">
                    <Globe className="h-4 w-4" />
                  </Button>
                </a>
              )}
              {isAuthenticated && (
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => router.push("/messages")}
                >
                  <MessageCircle className="h-4 w-4" />
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Services */}
        {services.length > 0 && (
          <Card className="border border-border">
            <CardHeader className="pb-3">
              <CardTitle className="text-base">Services offered</CardTitle>
            </CardHeader>
            <CardContent className="p-0">
              {services.map((svc, i) => (
                <div key={svc.id}>
                  {i > 0 && <Separator />}
                  <button
                    type="button"
                    onClick={() => {
                      setSelectedService(svc);
                      handleBook();
                    }}
                    className="w-full flex items-center justify-between px-5 py-4 hover:bg-muted/50 transition-colors text-left"
                  >
                    <div>
                      <p className="text-sm font-medium text-foreground">
                        {svc.title}
                      </p>
                      {svc.description && (
                        <p className="text-xs text-muted-foreground mt-0.5">
                          {svc.description}
                        </p>
                      )}
                    </div>
                    <div className="text-right shrink-0 ml-4">
                      {svc.price != null && (
                        <p className="text-sm font-semibold text-foreground">
                          KES {svc.price.toLocaleString()}
                        </p>
                      )}
                      {svc.duration != null && (
                        <p className="text-xs text-muted-foreground flex items-center gap-1 justify-end">
                          <Clock className="h-3 w-3" />
                          {svc.duration} min
                        </p>
                      )}
                    </div>
                  </button>
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {/* Reviews */}
        <Card className="border border-border">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">
              Reviews
              {reviewCount > 0 && (
                <span className="ml-2 text-sm font-normal text-muted-foreground">
                  ({reviewCount})
                </span>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {reviews.length === 0 ? (
              <p className="text-sm text-muted-foreground">No reviews yet.</p>
            ) : (
              reviews.map((r) => (
                <div key={r.id} className="space-y-1">
                  <div className="flex items-center gap-2">
                    <div className="flex gap-0.5">
                      {Array.from({ length: 5 }).map((_, i) => (
                        <Star
                          key={i}
                          className={`h-3.5 w-3.5 ${
                            i < r.rating
                              ? "fill-accent text-accent"
                              : "text-border fill-border"
                          }`}
                        />
                      ))}
                    </div>
                    {r.verified && (
                      <span className="flex items-center gap-1 text-xs text-primary">
                        <CheckCircle className="h-3 w-3" />
                        Verified booking
                      </span>
                    )}
                    <span className="text-xs text-muted-foreground ml-auto">
                      {format(new Date(r.created_at), "d MMM yyyy")}
                    </span>
                  </div>
                  <p className="text-sm text-foreground">{r.comment}</p>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>

      {/* Booking dialog */}
      <Dialog open={bookingOpen} onOpenChange={setBookingOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Book {provider.service_name}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 pt-2">
            {selectedService && (
              <div className="flex items-center justify-between px-3 py-2 bg-muted rounded-lg text-sm">
                <span className="font-medium">{selectedService.title}</span>
                {selectedService.price != null && (
                  <span className="text-foreground font-semibold">
                    KES {selectedService.price.toLocaleString()}
                  </span>
                )}
              </div>
            )}

            <div className="space-y-1.5">
              <Label>Date &amp; time</Label>
              <Input
                type="datetime-local"
                value={bookingForm.scheduled_time}
                onChange={(e) =>
                  setBookingForm((f) => ({ ...f, scheduled_time: e.target.value }))
                }
                min={new Date().toISOString().slice(0, 16)}
              />
            </div>

            <div className="space-y-1.5">
              <Label>Describe the work needed</Label>
              <Textarea
                placeholder="e.g. Fix leaking kitchen sink pipe"
                rows={3}
                value={bookingForm.service_description}
                onChange={(e) =>
                  setBookingForm((f) => ({ ...f, service_description: e.target.value }))
                }
              />
            </div>

            <div className="space-y-1.5">
              <Label>Your address</Label>
              <Input
                placeholder="e.g. Apt 4B, Ngong Road, Nairobi"
                value={bookingForm.client_address}
                onChange={(e) =>
                  setBookingForm((f) => ({ ...f, client_address: e.target.value }))
                }
              />
            </div>

            <div className="space-y-1.5">
              <Label>Your phone (for SMS updates)</Label>
              <Input
                type="tel"
                placeholder="07XX XXX XXX"
                value={bookingForm.client_phone}
                onChange={(e) =>
                  setBookingForm((f) => ({ ...f, client_phone: e.target.value }))
                }
              />
            </div>

            <Button
              className="w-full"
              onClick={submitBooking}
              disabled={bookingLoading}
            >
              {bookingLoading ? "Submitting…" : "Confirm booking"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

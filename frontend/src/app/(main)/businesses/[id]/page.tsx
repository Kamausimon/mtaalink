"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { api, type BusinessProfile, type Service, type Branch, type Review } from "@/lib/api";
import { useAuthStore } from "@/store/auth";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Star, MapPin, Phone, Globe, MessageCircle,
  CheckCircle, Clock, Building2,
} from "lucide-react";
import { toast } from "sonner";
import { format } from "date-fns";

export default function BusinessProfilePage() {
  const { id } = useParams<{ id: string }>();
  const router = useRouter();
  const { token, isAuthenticated } = useAuthStore();

  const [business, setBusiness] = useState<BusinessProfile | null>(null);
  const [services, setServices] = useState<Service[]>([]);
  const [branches, setBranches] = useState<Branch[]>([]);
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
          api.businesses.getById(Number(id)),
          api.reviews.get("business", Number(id)),
        ]);
        setBusiness(profileRes.business);
        setServices(profileRes.services);
        setBranches(profileRes.branches);
        setReviews(reviewRes.reviews);
      } catch {
        toast.error("Business not found");
        router.push("/search");
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [id, router]);

  async function handleBook() {
    if (!isAuthenticated) {
      router.push(`/login?next=/businesses/${id}`);
      return;
    }
    setBookingOpen(true);
  }

  async function submitBooking() {
    if (!bookingForm.scheduled_time) {
      toast.error("Please pick a date and time");
      return;
    }
    setBookingLoading(true);
    try {
      const res = await api.bookings.create(
        {
          target_type: "business",
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
      toast.success(`Booking #${res.booking_id} created!`);
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

  if (!business) return null;

  const avgRating = business.avg_rating;
  const reviewCount = business.review_count ?? 0;

  return (
    <>
      <div className="mx-auto max-w-3xl px-4 sm:px-6 py-8 space-y-6">
        {/* Header card */}
        <Card className="border border-border overflow-hidden">
          {business.cover_photo && (
            // eslint-disable-next-line @next/next/no-img-element
            <img src={business.cover_photo} alt="Cover" className="w-full h-36 object-cover" />
          )}
          <CardContent className="p-5">
            <div className="flex items-start gap-4">
              <Avatar className="h-16 w-16 border-2 border-white shadow-sm -mt-10 relative">
                <AvatarImage src={business.logo ?? business.profile_photo ?? undefined} />
                <AvatarFallback className="bg-primary/10 text-primary font-bold text-lg">
                  {business.business_name?.slice(0, 2).toUpperCase()}
                </AvatarFallback>
              </Avatar>
              <div className="flex-1 min-w-0">
                <h1 className="text-xl font-bold text-foreground">{business.business_name}</h1>
                <div className="flex flex-wrap items-center gap-3 mt-1">
                  {business.category && <Badge variant="secondary">{business.category}</Badge>}
                  {avgRating != null && (
                    <div className="flex items-center gap-1 text-sm">
                      <Star className="h-4 w-4 fill-accent text-accent" />
                      <span className="font-medium">{avgRating.toFixed(1)}</span>
                      <span className="text-muted-foreground">({reviewCount} reviews)</span>
                    </div>
                  )}
                  {business.location && (
                    <div className="flex items-center gap-1 text-sm text-muted-foreground">
                      <MapPin className="h-3.5 w-3.5" />
                      {business.location}
                    </div>
                  )}
                </div>
              </div>
            </div>

            {business.description && (
              <p className="mt-4 text-sm text-muted-foreground leading-relaxed">
                {business.description}
              </p>
            )}

            <div className="flex flex-wrap items-center gap-3 mt-5">
              <Button onClick={handleBook} className="gap-2">Book this business</Button>
              {business.phone_number && (
                <a href={`tel:${business.phone_number}`}>
                  <Button variant="outline" size="icon"><Phone className="h-4 w-4" /></Button>
                </a>
              )}
              {business.website && (
                <a href={business.website} target="_blank" rel="noopener noreferrer">
                  <Button variant="outline" size="icon"><Globe className="h-4 w-4" /></Button>
                </a>
              )}
              {isAuthenticated && (
                <Button variant="outline" size="icon" onClick={() => router.push("/messages")}>
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
                    onClick={() => { setSelectedService(svc); handleBook(); }}
                    className="w-full flex items-center justify-between px-5 py-4 hover:bg-muted/50 transition-colors text-left"
                  >
                    <div>
                      <p className="text-sm font-medium text-foreground">{svc.title}</p>
                      {svc.description && (
                        <p className="text-xs text-muted-foreground mt-0.5">{svc.description}</p>
                      )}
                    </div>
                    <div className="text-right shrink-0 ml-4">
                      {svc.price != null && (
                        <p className="text-sm font-semibold">KES {svc.price.toLocaleString()}</p>
                      )}
                      {svc.duration != null && (
                        <p className="text-xs text-muted-foreground flex items-center gap-1 justify-end">
                          <Clock className="h-3 w-3" />{svc.duration} min
                        </p>
                      )}
                    </div>
                  </button>
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {/* Branches */}
        {branches.length > 0 && (
          <Card className="border border-border">
            <CardHeader className="pb-3">
              <CardTitle className="text-base">Our locations</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {branches.map((b) => (
                <div key={b.id} className="flex items-start gap-3 p-3 bg-muted/30 rounded-lg">
                  <Building2 className="h-4 w-4 text-primary shrink-0 mt-0.5" />
                  <div>
                    <p className="text-sm font-medium text-foreground">{b.name}</p>
                    {b.address && (
                      <p className="text-xs text-muted-foreground mt-0.5">{b.address}</p>
                    )}
                    {(b.ward || b.constituency || b.county) && (
                      <p className="text-xs text-muted-foreground">
                        {[b.ward, b.constituency, b.county].filter(Boolean).join(", ")}
                      </p>
                    )}
                    {b.phone && (
                      <a href={`tel:${b.phone}`} className="text-xs text-primary mt-1 block">
                        {b.phone}
                      </a>
                    )}
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {/* Reviews */}
        <Card className="border border-border">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">
              Reviews {reviewCount > 0 && <span className="text-sm font-normal text-muted-foreground">({reviewCount})</span>}
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
                        <Star key={i} className={`h-3.5 w-3.5 ${i < r.rating ? "fill-accent text-accent" : "text-border fill-border"}`} />
                      ))}
                    </div>
                    {r.verified && (
                      <span className="flex items-center gap-1 text-xs text-primary">
                        <CheckCircle className="h-3 w-3" />Verified booking
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
            <DialogTitle>Book {business.business_name}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 pt-2">
            {selectedService && (
              <div className="flex items-center justify-between px-3 py-2 bg-muted rounded-lg text-sm">
                <span className="font-medium">{selectedService.title}</span>
                {selectedService.price != null && (
                  <span className="font-semibold">KES {selectedService.price.toLocaleString()}</span>
                )}
              </div>
            )}
            <div className="space-y-1.5">
              <Label>Date &amp; time</Label>
              <Input type="datetime-local" value={bookingForm.scheduled_time}
                onChange={(e) => setBookingForm((f) => ({ ...f, scheduled_time: e.target.value }))}
                min={new Date().toISOString().slice(0, 16)} />
            </div>
            <div className="space-y-1.5">
              <Label>Describe what you need</Label>
              <Textarea placeholder="e.g. Deep clean 3-bedroom apartment" rows={3}
                value={bookingForm.service_description}
                onChange={(e) => setBookingForm((f) => ({ ...f, service_description: e.target.value }))} />
            </div>
            <div className="space-y-1.5">
              <Label>Your address</Label>
              <Input placeholder="e.g. Apt 4B, Ngong Road, Nairobi" value={bookingForm.client_address}
                onChange={(e) => setBookingForm((f) => ({ ...f, client_address: e.target.value }))} />
            </div>
            <div className="space-y-1.5">
              <Label>Your phone (for SMS updates)</Label>
              <Input type="tel" placeholder="07XX XXX XXX" value={bookingForm.client_phone}
                onChange={(e) => setBookingForm((f) => ({ ...f, client_phone: e.target.value }))} />
            </div>
            <Button className="w-full" onClick={submitBooking} disabled={bookingLoading}>
              {bookingLoading ? "Submitting…" : "Confirm booking"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

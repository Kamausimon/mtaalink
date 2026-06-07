"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type ProviderProfile, type BusinessProfile } from "@/lib/api";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Heart, Star, MapPin, ArrowRight } from "lucide-react";
import { toast } from "sonner";

type FavoriteItem =
  | { type: "provider"; id: number; data: ProviderProfile }
  | { type: "business"; id: number; data: BusinessProfile };

export default function FavoritesPage() {
  const { token, isAuthenticated, user, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [items, setItems] = useState<FavoriteItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [removingId, setRemovingId] = useState<string | null>(null);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.push("/login"); return; }
    if (user?.role !== "client") { router.push("/dashboard"); return; }
    loadFavorites();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated, user?.role]);

  async function loadFavorites() {
    setLoading(true);
    try {
      const { favorites } = await api.favorites.list(token!);
      if (favorites.length === 0) { setItems([]); return; }

      const results = await Promise.allSettled(
        favorites.map(async (f) => {
          if (f.target_type === "provider") {
            const res = await api.providers.getById(f.target_id);
            return { type: "provider" as const, id: f.target_id, data: res.provider };
          } else {
            const res = await api.businesses.getById(f.target_id);
            return { type: "business" as const, id: f.target_id, data: res.business };
          }
        }),
      );

      setItems(
        results
          .filter((r): r is PromiseFulfilledResult<FavoriteItem> => r.status === "fulfilled")
          .map((r) => r.value),
      );
    } catch {
      toast.error("Could not load favourites");
    } finally {
      setLoading(false);
    }
  }

  async function remove(item: FavoriteItem) {
    const key = `${item.type}-${item.id}`;
    setRemovingId(key);
    try {
      await api.favorites.remove(item.id, item.type, token!);
      setItems((prev) => prev.filter((i) => !(i.type === item.type && i.id === item.id)));
      toast.success("Removed from favourites");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Could not remove");
    } finally {
      setRemovingId(null);
    }
  }

  function navigate(item: FavoriteItem) {
    router.push(item.type === "provider" ? `/providers/${item.id}` : `/businesses/${item.id}`);
  }

  const name = (item: FavoriteItem) =>
    item.type === "provider"
      ? (item.data as ProviderProfile).service_name ?? "Provider"
      : (item.data as BusinessProfile).business_name;

  const photo = (item: FavoriteItem) =>
    item.type === "provider"
      ? (item.data as ProviderProfile).profile_photo
      : (item.data as BusinessProfile).logo ?? (item.data as BusinessProfile).profile_photo;

  if (!_hasHydrated) return null;

  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 py-8 space-y-6">
      <h1 className="text-2xl font-bold text-foreground">Favourites</h1>

      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} className="h-24 rounded-xl" />)}
        </div>
      ) : items.length === 0 ? (
        <div className="text-center py-20">
          <Heart className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
          <p className="font-medium text-foreground mb-1">No favourites yet</p>
          <p className="text-sm text-muted-foreground mb-6">
            Tap the heart icon on any provider or business to save them here.
          </p>
          <Button onClick={() => router.push("/search")}>Find providers</Button>
        </div>
      ) : (
        <div className="space-y-3">
          {items.map((item) => {
            const key = `${item.type}-${item.id}`;
            const isRemoving = removingId === key;
            return (
              <Card key={key} className="border border-border hover:border-primary/40 transition-colors">
                <CardContent className="p-4">
                  <div className="flex items-center gap-3">
                    <Avatar className="h-12 w-12 shrink-0">
                      <AvatarImage src={photo(item) ?? undefined} />
                      <AvatarFallback className="bg-primary/10 text-primary font-semibold text-sm">
                        {name(item).slice(0, 2).toUpperCase()}
                      </AvatarFallback>
                    </Avatar>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <p className="text-sm font-semibold text-foreground truncate">{name(item)}</p>
                        <Badge variant="secondary" className="text-xs capitalize shrink-0">{item.type}</Badge>
                      </div>

                      <div className="flex flex-wrap items-center gap-3 mt-0.5 text-xs text-muted-foreground">
                        {item.data.category && (
                          <span>{item.data.category}</span>
                        )}
                        {item.data.avg_rating != null && item.data.avg_rating > 0 && (
                          <span className="flex items-center gap-0.5">
                            <Star className="h-3 w-3 fill-amber-400 text-amber-400" />
                            {item.data.avg_rating.toFixed(1)}
                            {item.data.review_count != null && item.data.review_count > 0 && (
                              <span className="text-muted-foreground/70">({item.data.review_count})</span>
                            )}
                          </span>
                        )}
                        {item.data.location && (
                          <span className="flex items-center gap-0.5">
                            <MapPin className="h-3 w-3" />{item.data.location}
                          </span>
                        )}
                      </div>
                    </div>

                    <div className="flex items-center gap-1 shrink-0">
                      <Button
                        variant="ghost"
                        size="icon"
                        disabled={isRemoving}
                        onClick={() => remove(item)}
                        title="Remove from favourites"
                        className="h-8 w-8 text-rose-500 hover:text-rose-600 hover:bg-rose-50"
                      >
                        <Heart className="h-4 w-4 fill-rose-500" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8"
                        onClick={() => navigate(item)}
                        title="View profile"
                      >
                        <ArrowRight className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

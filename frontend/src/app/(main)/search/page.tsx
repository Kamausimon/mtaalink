"use client";

import { useEffect, useState, useCallback, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import Link from "next/link";
import { useAuthStore } from "@/store/auth";
import { api, type SearchResult } from "@/lib/api";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Search, SlidersHorizontal, Star, MapPin, Loader2, Navigation } from "lucide-react";
import { toast } from "sonner";

const PER_PAGE = 12;

function ResultCard({ result }: { result: SearchResult }) {
  const href = result.type === "business" ? `/businesses/${result.id}` : `/providers/${result.id}`;
  const initials = result.name?.slice(0, 2).toUpperCase() ?? "??";

  return (
    <Link href={href}>
      <Card className="border border-border hover:border-primary hover:shadow-sm transition-all cursor-pointer h-full">
        <CardContent className="p-4 flex flex-col gap-3">
          <div className="flex items-start gap-3">
            <Avatar className="h-12 w-12 shrink-0">
              <AvatarImage src={result.profile_photo ?? undefined} alt={result.name ?? ""} />
              <AvatarFallback className="bg-primary/10 text-primary font-semibold text-sm">
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="flex-1 min-w-0">
              <h3 className="font-semibold text-foreground text-sm leading-tight truncate">
                {result.name}
              </h3>
              <div className="flex items-center gap-1.5 mt-1 flex-wrap">
                {result.category && (
                  <Badge variant="secondary" className="text-xs font-normal">
                    {result.category}
                  </Badge>
                )}
                {result.type === "business" && (
                  <Badge variant="outline" className="text-xs font-normal">Business</Badge>
                )}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3">
            {result.location && (
              <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <MapPin className="h-3 w-3 shrink-0" />
                <span className="truncate">{result.location}</span>
              </div>
            )}
            {result.distance_km != null && (
              <span className="text-xs text-primary font-medium ml-auto shrink-0">
                {result.distance_km < 1
                  ? `${Math.round(result.distance_km * 1000)} m away`
                  : `${result.distance_km.toFixed(1)} km away`}
              </span>
            )}
          </div>

          <div className="flex items-center justify-between mt-auto pt-1 border-t border-border">
            {result.avg_rating != null ? (
              <div className="flex items-center gap-1">
                <Star className="h-3.5 w-3.5 fill-accent text-accent" />
                <span className="text-sm font-medium">{result.avg_rating.toFixed(1)}</span>
                <span className="text-xs text-muted-foreground">
                  ({result.review_count ?? 0} {(result.review_count ?? 0) === 1 ? "review" : "reviews"})
                </span>
              </div>
            ) : (
              <span className="text-xs text-muted-foreground">No reviews yet</span>
            )}
          </div>
        </CardContent>
      </Card>
    </Link>
  );
}

function SearchContent() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const { user, _hasHydrated } = useAuthStore();

  const [query, setQuery] = useState(searchParams.get("q") ?? "");
  const [category, setCategory] = useState(searchParams.get("category") ?? "");
  const [location, setLocation] = useState(searchParams.get("location") ?? "");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [searched, setSearched] = useState(false);
  const [userLat, setUserLat] = useState<number | null>(null);
  const [userLng, setUserLng] = useState<number | null>(null);
  const [gpsLoading, setGpsLoading] = useState(false);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (user && user.role !== "client") router.replace("/dashboard");
  }, [_hasHydrated, user, router]);

  function useMyLocation() {
    if (!navigator.geolocation) {
      toast.error("Your browser doesn't support location. Try typing your area instead.");
      return;
    }
    setGpsLoading(true);
    navigator.geolocation.getCurrentPosition(
      (pos) => {
        const lat = pos.coords.latitude;
        const lng = pos.coords.longitude;
        setUserLat(lat);
        setUserLng(lng);
        setGpsLoading(false);
        runSearch(query, category, location, 1, false, lat, lng);
      },
      (err) => {
        setGpsLoading(false);
        if (err.code === err.PERMISSION_DENIED) {
          toast.error(
            "Location access is blocked. Enable it in your browser settings, or just type your area (e.g. \"Kasarani, Nairobi\") in the location field.",
            { duration: 6000 },
          );
        } else {
          toast.error("Could not get your location. Try typing your area instead.");
        }
      },
      { timeout: 10000 },
    );
  }

  const runSearch = useCallback(async (q: string, cat: string, loc: string, nextPage = 1, append = false, lat?: number, lng?: number) => {
    if (nextPage === 1) setLoading(true);
    else setLoadingMore(true);
    setSearched(true);

    const resolvedLat = lat ?? userLat ?? undefined;
    const resolvedLng = lng ?? userLng ?? undefined;

    try {
      const res = await api.search.query({
        q: q || undefined,
        category: cat || undefined,
        location: loc || undefined,
        page: nextPage,
        per_page: PER_PAGE,
        lat: resolvedLat ?? undefined,
        lng: resolvedLng ?? undefined,
        radius_km: resolvedLat ? 15 : undefined,
      });
      setResults((prev) => (append ? [...prev, ...res.results] : res.results));
      setTotal(res.total);
      setPage(nextPage);
    } catch {
      if (!append) setResults([]);
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, []);

  useEffect(() => {
    const q = searchParams.get("q");
    const cat = searchParams.get("category");
    const loc = searchParams.get("location");
    setQuery(q ?? "");
    setCategory(cat ?? "");
    setLocation(loc ?? "");
    runSearch(q ?? "", cat ?? "", loc ?? "", 1, false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const qs = new URLSearchParams();
    if (query) qs.set("q", query);
    if (category) qs.set("category", category);
    if (location) qs.set("location", location);
    router.push(`/search?${qs}`);
    runSearch(query, category, location, 1, false);
  }

  function selectCategory(cat: string) {
    setCategory(cat);
    runSearch(query, cat, location, 1, false);
  }

  function loadMore() {
    runSearch(query, category, location, page + 1, true);
  }

  const hasMore = results.length < total;

  const CATEGORIES = [
    "Plumbing", "Electrical", "Cleaning", "Tutoring",
    "Painting", "Security", "Catering", "Beauty",
  ];

  return (
    <div className="mx-auto max-w-6xl px-4 sm:px-6 py-8">
      {/* Search bar */}
      <form onSubmit={handleSubmit} className="flex flex-col sm:flex-row gap-2 mb-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="What are you looking for? e.g. Plumbing"
            className="pl-9 bg-white"
          />
        </div>
        <div className="relative flex-1">
          <MapPin className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            value={location}
            onChange={(e) => setLocation(e.target.value)}
            placeholder="Location e.g. Kasarani, Nairobi"
            className="pl-9 bg-white"
          />
        </div>
        <div className="flex gap-2">
          <Button
            type="button"
            variant={userLat ? "default" : "outline"}
            className="gap-1.5 shrink-0"
            onClick={useMyLocation}
            disabled={gpsLoading}
            title="Search near my current location"
          >
            {gpsLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Navigation className="h-4 w-4" />
            )}
            <span className="hidden sm:inline">{userLat ? "Near me ✓" : "Near me"}</span>
          </Button>
          <Button type="submit" className="shrink-0">Search</Button>
        </div>
      </form>
      <p className="text-xs text-muted-foreground mb-6">
        Tip: type an area like &ldquo;Kasarani, Nairobi&rdquo; if you&apos;d rather not share your location.
      </p>

      {/* Category pills */}
      <div className="flex flex-wrap gap-2 mb-8">
        <button
          type="button"
          onClick={() => selectCategory("")}
          className={`px-3 py-1.5 rounded-full text-xs font-medium border transition-colors ${
            !category
              ? "bg-primary text-white border-primary"
              : "bg-white text-foreground border-border hover:border-primary"
          }`}
        >
          All
        </button>
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            type="button"
            onClick={() => selectCategory(cat)}
            className={`px-3 py-1.5 rounded-full text-xs font-medium border transition-colors ${
              category === cat
                ? "bg-primary text-white border-primary"
                : "bg-white text-foreground border-border hover:border-primary"
            }`}
          >
            {cat}
          </button>
        ))}
      </div>

      {/* Results */}
      {loading ? (
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-36 rounded-lg" />
          ))}
        </div>
      ) : results.length > 0 ? (
        <>
          <p className="text-sm text-muted-foreground mb-4">
            Showing {results.length} of {total} result{total !== 1 ? "s" : ""}
            {(query || category || location) && (
              <span>
                {query && <> for <strong>&ldquo;{query}&rdquo;</strong></>}
                {category && <> in <strong>{category}</strong></>}
                {location && <> near <strong>{location}</strong></>}
              </span>
            )}
          </p>
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {results.map((r) => (
              <ResultCard key={`${r.type}-${r.id}`} result={r} />
            ))}
          </div>

          {hasMore && (
            <div className="flex justify-center mt-8">
              <Button
                variant="outline"
                onClick={loadMore}
                disabled={loadingMore}
                className="gap-2 min-w-32"
              >
                {loadingMore ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Loading…
                  </>
                ) : (
                  `Load more (${total - results.length} remaining)`
                )}
              </Button>
            </div>
          )}
        </>
      ) : searched ? (
        <div className="text-center py-16">
          <SlidersHorizontal className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
          <p className="font-medium text-foreground mb-1">No results found</p>
          <p className="text-sm text-muted-foreground">
            Try different keywords or clear the category filter.
          </p>
        </div>
      ) : null}
    </div>
  );
}

export default function SearchPage() {
  return (
    <Suspense>
      <SearchContent />
    </Suspense>
  );
}

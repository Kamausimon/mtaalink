"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { api, type PublicProvider } from "@/lib/api";
import ProviderCard from "@/components/ProviderCard";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Search, SlidersHorizontal } from "lucide-react";

function SearchContent() {
  const searchParams = useSearchParams();
  const router = useRouter();

  const [query, setQuery] = useState(searchParams.get("q") ?? "");
  const [category, setCategory] = useState(searchParams.get("category") ?? "");
  const [providers, setProviders] = useState<PublicProvider[]>([]);
  const [loading, setLoading] = useState(false);
  const [searched, setSearched] = useState(false);

  useEffect(() => {
    const q = searchParams.get("q");
    const cat = searchParams.get("category");
    if (q || cat) {
      setQuery(q ?? "");
      setCategory(cat ?? "");
      runSearch(q ?? "", cat ?? "");
    } else {
      // Show all providers by default
      runSearch("", "");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function runSearch(q: string, cat: string) {
    setLoading(true);
    setSearched(true);
    try {
      const res = await api.providers.list({
        category: cat || undefined,
        location: q || undefined,
      });
      setProviders(res.providers);
    } catch {
      setProviders([]);
    } finally {
      setLoading(false);
    }
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const qs = new URLSearchParams();
    if (query) qs.set("q", query);
    if (category) qs.set("category", category);
    router.push(`/search?${qs}`);
    runSearch(query, category);
  }

  const CATEGORIES = [
    "Plumbing", "Electrical", "Cleaning", "Tutoring",
    "Painting", "Security", "Catering", "Beauty",
  ];

  return (
    <div className="mx-auto max-w-6xl px-4 sm:px-6 py-8">
      {/* Search bar */}
      <form onSubmit={handleSubmit} className="flex gap-2 mb-6">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Service or location…"
            className="pl-9 bg-white"
          />
        </div>
        <Button type="submit">Search</Button>
      </form>

      {/* Category pills */}
      <div className="flex flex-wrap gap-2 mb-8">
        <button
          type="button"
          onClick={() => { setCategory(""); runSearch(query, ""); }}
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
            onClick={() => { setCategory(cat); runSearch(query, cat); }}
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
      ) : providers.length > 0 ? (
        <>
          <p className="text-sm text-muted-foreground mb-4">
            {providers.length} provider{providers.length !== 1 ? "s" : ""} found
          </p>
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {providers.map((p) => (
              <ProviderCard key={p.id} provider={p} />
            ))}
          </div>
        </>
      ) : searched ? (
        <div className="text-center py-16">
          <SlidersHorizontal className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
          <p className="font-medium text-foreground mb-1">No providers found</p>
          <p className="text-sm text-muted-foreground">
            Try a different search or clear the category filter.
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

"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type FlaggedReview, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { toast } from "sonner";
import { ShieldCheck, Star, Flag } from "lucide-react";
import { cn } from "@/lib/utils";

function StarRating({ rating }: { rating: number }) {
  return (
    <span className="flex items-center gap-0.5">
      {Array.from({ length: 5 }).map((_, i) => (
        <Star
          key={i}
          className={cn("h-3.5 w-3.5", i < rating ? "fill-yellow-400 text-yellow-400" : "text-muted-foreground/30")}
        />
      ))}
    </span>
  );
}

export default function AdminModerationPage() {
  const { token, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [reviews, setReviews] = useState<FlaggedReview[]>([]);
  const [loading, setLoading] = useState(true);
  const [resolving, setResolving] = useState<number | null>(null);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    api.admin.flaggedReviews(token)
      .then((r) => setReviews(r.flagged_reviews))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 403) router.replace("/dashboard");
        else toast.error("Failed to load flagged reviews");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  async function handleResolve(reviewId: number) {
    if (!token) return;
    setResolving(reviewId);
    try {
      await api.admin.resolveFlag(reviewId, token);
      setReviews((r) => r.filter((x) => x.review_id !== reviewId));
      toast.success("Flags resolved — review cleared");
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) {
        setReviews((r) => r.filter((x) => x.review_id !== reviewId));
        toast.info("Already resolved");
      } else {
        toast.error("Failed to resolve flags");
      }
    } finally {
      setResolving(null);
    }
  }

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-4 max-w-3xl">
        <Skeleton className="h-8 w-40" />
        {Array.from({ length: 4 }).map((_, i) => <Skeleton key={i} className="h-28 rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-5 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Content Moderation</h1>
        <p className="text-sm text-muted-foreground mt-1">
          {reviews.length === 0
            ? "No flagged reviews."
            : `${reviews.length} review${reviews.length === 1 ? "" : "s"} with active flags`}
        </p>
      </div>

      {reviews.length === 0 ? (
        <div className="rounded-xl border border-border bg-white px-6 py-12 text-center">
          <ShieldCheck className="h-8 w-8 text-green-500 mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">All clear — no flagged content.</p>
        </div>
      ) : (
        <div className="rounded-xl border border-border overflow-hidden bg-white">
          {reviews.map((r, i) => (
            <div key={r.review_id}>
              {i > 0 && <Separator />}
              <div className="px-5 py-4 space-y-2">
                <div className="flex items-start justify-between gap-4">
                  <div className="space-y-1 min-w-0">
                    <div className="flex items-center gap-3 flex-wrap">
                      <StarRating rating={r.rating} />
                      <span className="text-xs text-muted-foreground">
                        on {r.target_type} #{r.target_id}
                      </span>
                      <span className="flex items-center gap-1 text-xs font-semibold text-red-600">
                        <Flag className="h-3 w-3" />
                        {r.flag_count} flag{Number(r.flag_count) !== 1 ? "s" : ""}
                      </span>
                    </div>
                    {r.comment ? (
                      <p className="text-sm text-foreground">{r.comment}</p>
                    ) : (
                      <p className="text-sm text-muted-foreground italic">No comment</p>
                    )}
                    <p className="text-xs text-muted-foreground">
                      Reviewer ID: {r.reviewer_id} · Review #{r.review_id}
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="outline"
                    className="shrink-0 text-green-700 border-green-200 hover:bg-green-50"
                    disabled={resolving === r.review_id}
                    onClick={() => handleResolve(r.review_id)}
                  >
                    <ShieldCheck className="h-4 w-4 mr-1" />
                    {resolving === r.review_id ? "Resolving…" : "Resolve flags"}
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

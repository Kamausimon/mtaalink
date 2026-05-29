import Link from "next/link";
import { Star, MapPin } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import type { PublicProvider } from "@/lib/api";

export default function ProviderCard({ provider }: { provider: PublicProvider }) {
  const initials = provider.service_name?.slice(0, 2).toUpperCase() ?? "??";
  const rating = provider.avg_rating;
  const reviewCount = provider.review_count ?? 0;

  return (
    <Link href={`/providers/${provider.id}`}>
      <Card className="border border-border hover:border-primary hover:shadow-sm transition-all cursor-pointer h-full">
        <CardContent className="p-4 flex flex-col gap-3">
          <div className="flex items-start gap-3">
            <Avatar className="h-12 w-12 shrink-0">
              <AvatarImage src={provider.profile_photo ?? undefined} alt={provider.service_name ?? ""} />
              <AvatarFallback className="bg-primary/10 text-primary font-semibold text-sm">
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="flex-1 min-w-0">
              <h3 className="font-semibold text-foreground text-sm leading-tight truncate">
                {provider.service_name}
              </h3>
              {provider.category && (
                <Badge variant="secondary" className="mt-1 text-xs font-normal">
                  {provider.category}
                </Badge>
              )}
            </div>
          </div>

          {provider.location && (
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <MapPin className="h-3 w-3 shrink-0" />
              <span className="truncate">{provider.location}</span>
            </div>
          )}

          <div className="flex items-center justify-between mt-auto pt-1 border-t border-border">
            {rating != null ? (
              <div className="flex items-center gap-1">
                <Star className="h-3.5 w-3.5 fill-accent text-accent" />
                <span className="text-sm font-medium">{rating.toFixed(1)}</span>
                <span className="text-xs text-muted-foreground">
                  ({reviewCount} {reviewCount === 1 ? "review" : "reviews"})
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

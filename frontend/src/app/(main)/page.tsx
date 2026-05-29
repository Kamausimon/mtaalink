import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Search,
  CheckCircle,
  Star,
  Shield,
  Smartphone,
  ArrowRight,
} from "lucide-react";

const POPULAR_CATEGORIES = [
  { label: "Plumbing", emoji: "🔧" },
  { label: "Electrical", emoji: "⚡" },
  { label: "Cleaning", emoji: "🧹" },
  { label: "Tutoring", emoji: "📚" },
  { label: "Painting", emoji: "🎨" },
  { label: "Security", emoji: "🔒" },
  { label: "Catering", emoji: "🍽️" },
  { label: "Beauty", emoji: "💅" },
];

const HOW_IT_WORKS = [
  {
    step: "1",
    title: "Search your area",
    description:
      "Type the service you need — plumber, cleaner, tutor — and we'll show you providers near you.",
  },
  {
    step: "2",
    title: "Pick a provider",
    description:
      "Read reviews from verified clients, check their services and pricing, then book directly.",
  },
  {
    step: "3",
    title: "Pay with M-Pesa",
    description:
      "Confirm the booking and pay securely with M-Pesa. No cash, no chasing — done.",
  },
];

const TRUST_POINTS = [
  {
    icon: <CheckCircle className="h-5 w-5 text-primary" />,
    title: "Verified bookings only",
    description: "Reviews are only allowed after a completed booking — no fake ratings.",
  },
  {
    icon: <Shield className="h-5 w-5 text-primary" />,
    title: "Secure payments",
    description: "All payments go through M-Pesa STK Push. Your money is safe.",
  },
  {
    icon: <Smartphone className="h-5 w-5 text-primary" />,
    title: "SMS + in-app updates",
    description: "Get notified by SMS and in-app when your booking is confirmed or changed.",
  },
  {
    icon: <Star className="h-5 w-5 text-primary" />,
    title: "Real ratings",
    description: "Every rating comes from a client who actually used the service.",
  },
];

export default function LandingPage() {
  return (
    <div className="flex flex-col">
      {/* ── Hero ──────────────────────────────────────────────────────── */}
      <section className="bg-white border-b border-border">
        <div className="mx-auto max-w-6xl px-4 sm:px-6 py-16 sm:py-24">
          <div className="max-w-2xl">
            <Badge className="mb-4 bg-primary/10 text-primary border-primary/20 hover:bg-primary/10">
              Nairobi &amp; beyond
            </Badge>
            <h1 className="text-4xl sm:text-5xl font-bold text-foreground leading-tight mb-4">
              Find trusted local services in your{" "}
              <span className="text-primary">neighbourhood</span>
            </h1>
            <p className="text-lg text-muted-foreground mb-8">
              Book plumbers, cleaners, tutors, electricians, and more — all
              reviewed by real clients and paid by M-Pesa.
            </p>

            {/* Search bar */}
            <form
              action="/search"
              method="GET"
              className="flex gap-2 max-w-xl"
            >
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  name="q"
                  placeholder="What service do you need?"
                  className="pl-9 h-11 bg-white"
                />
              </div>
              <Button type="submit" size="lg" className="h-11 px-6 shrink-0">
                Search
              </Button>
            </form>
          </div>
        </div>
      </section>

      {/* ── Popular categories ─────────────────────────────────────────── */}
      <section className="bg-background py-12">
        <div className="mx-auto max-w-6xl px-4 sm:px-6">
          <h2 className="text-lg font-semibold text-foreground mb-5">
            Popular categories
          </h2>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {POPULAR_CATEGORIES.map((cat) => (
              <Link
                key={cat.label}
                href={`/search?category=${encodeURIComponent(cat.label)}`}
                className="flex items-center gap-3 p-4 bg-white border border-border rounded-lg hover:border-primary hover:shadow-sm transition-all group"
              >
                <span className="text-2xl">{cat.emoji}</span>
                <span className="text-sm font-medium text-foreground group-hover:text-primary transition-colors">
                  {cat.label}
                </span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      {/* ── How it works ──────────────────────────────────────────────── */}
      <section className="bg-white border-t border-b border-border py-16">
        <div className="mx-auto max-w-6xl px-4 sm:px-6">
          <h2 className="text-2xl font-bold text-foreground mb-2">
            How MtaaLink works
          </h2>
          <p className="text-muted-foreground mb-10">
            From search to payment in three steps.
          </p>
          <div className="grid sm:grid-cols-3 gap-8">
            {HOW_IT_WORKS.map((item) => (
              <div key={item.step} className="flex flex-col">
                <div className="h-10 w-10 rounded-full bg-primary text-white flex items-center justify-center text-sm font-bold mb-4">
                  {item.step}
                </div>
                <h3 className="font-semibold text-foreground mb-2">
                  {item.title}
                </h3>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {item.description}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── Trust signals ─────────────────────────────────────────────── */}
      <section className="bg-background py-16">
        <div className="mx-auto max-w-6xl px-4 sm:px-6">
          <h2 className="text-2xl font-bold text-foreground mb-10">
            Why clients trust MtaaLink
          </h2>
          <div className="grid sm:grid-cols-2 gap-6">
            {TRUST_POINTS.map((point) => (
              <div
                key={point.title}
                className="flex gap-4 p-5 bg-white border border-border rounded-lg"
              >
                <div className="shrink-0 mt-0.5">{point.icon}</div>
                <div>
                  <h3 className="font-semibold text-foreground mb-1">
                    {point.title}
                  </h3>
                  <p className="text-sm text-muted-foreground">
                    {point.description}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── CTA for providers ─────────────────────────────────────────── */}
      <section className="bg-primary py-14">
        <div className="mx-auto max-w-6xl px-4 sm:px-6 flex flex-col sm:flex-row items-center justify-between gap-6">
          <div className="text-white">
            <h2 className="text-2xl font-bold mb-2">
              Are you a service provider?
            </h2>
            <p className="text-primary-foreground/80 text-sm">
              Join MtaaLink, set your prices, and start getting clients in your
              area today.
            </p>
          </div>
          <Link href="/register?role=provider" className="shrink-0">
            <Button
              variant="secondary"
              size="lg"
              className="gap-2 bg-white text-primary hover:bg-white/90"
            >
              Join as a provider
              <ArrowRight className="h-4 w-4" />
            </Button>
          </Link>
        </div>
      </section>
    </div>
  );
}

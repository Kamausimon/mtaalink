"use client";

import Link from "next/link";
import { useRouter, usePathname } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Bell, Menu, X, Search, Heart, ShieldCheck, Compass } from "lucide-react";
import { useState } from "react";
import { startSiteTour } from "@/lib/tour";

export default function Navbar() {
  const { user, isAuthenticated, isAdmin, clearAuth } = useAuthStore();
  const router = useRouter();
  const pathname = usePathname();
  const [mobileOpen, setMobileOpen] = useState(false);

  const initials = user?.username
    ? user.username.slice(0, 2).toUpperCase()
    : "??";

  function handleLogout() {
    clearAuth();
    router.push("/");
  }

  const isProvider = user?.role === "provider";
  const isBusiness = user?.role === "business";

  const navLinks =
    isProvider
      ? [
          { href: "/bookings", label: "Bookings" },
          { href: "/availability", label: "Availability" },
          { href: "/services", label: "Services" },
          { href: "/analytics", label: "Analytics" },
        ]
      : isBusiness
      ? [{ href: "/bookings", label: "Bookings" }, { href: "/services", label: "Services" }, { href: "/analytics", label: "Analytics" }]
      : [
          { href: "/search", label: "Find Services" },
          { href: "/favorites", label: "Favourites" },
        ];

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border bg-white">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <div className="flex h-16 items-center justify-between gap-4">
          {/* Logo */}
          <Link href={isAuthenticated ? "/dashboard" : "/"} className="flex items-center gap-2 shrink-0" data-tour="logo">
            <span className="text-xl font-bold text-primary tracking-tight">
              Sok<span className="text-accent">avi</span>
            </span>
          </Link>

          {/* Desktop nav */}
          <nav className="hidden md:flex items-center gap-6" data-tour="nav-links">
            {navLinks.map((l) => (
              <Link
                key={l.href}
                href={l.href}
                className={`text-sm font-medium transition-colors hover:text-primary ${
                  pathname.startsWith(l.href)
                    ? "text-primary"
                    : "text-muted-foreground"
                }`}
              >
                {l.label}
              </Link>
            ))}
          </nav>

          {/* Right side */}
          <div className="flex items-center gap-2">
            {isAuthenticated && user ? (
              <>
                {/* Notifications */}
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => router.push("/notifications")}
                  data-tour="notifications"
                >
                  <Bell className="h-5 w-5" />
                </Button>

                {/* User menu */}
                <DropdownMenu>
                  <DropdownMenuTrigger className="flex items-center gap-2 h-9 px-2 rounded-md hover:bg-muted transition-colors outline-none" data-tour="user-menu">
                    <Avatar className="h-8 w-8">
                      <AvatarFallback className="bg-primary text-white text-xs font-semibold">
                        {initials}
                      </AvatarFallback>
                    </Avatar>
                    <span className="hidden sm:block text-sm font-medium max-w-30 truncate">
                      {user.username}
                    </span>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-48">
                    <DropdownMenuItem onClick={() => router.push("/dashboard")}>
                      Dashboard
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => router.push("/bookings")}>
                      My Bookings
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => router.push("/messages")}>
                      Messages
                    </DropdownMenuItem>
                    {user.role === "client" && (
                      <DropdownMenuItem onClick={() => router.push("/favorites")} className="gap-2">
                        <Heart className="h-4 w-4" />
                        Favourites
                      </DropdownMenuItem>
                    )}
                    {(user.role === "provider" || user.role === "business") && (
                      <DropdownMenuItem onClick={() => router.push("/my-profile")}>
                        My Public Profile
                      </DropdownMenuItem>
                    )}
                    {(user.role === "provider" || user.role === "business") && (
                      <DropdownMenuItem onClick={() => router.push("/services")}>
                        Manage Services
                      </DropdownMenuItem>
                    )}
                    {(user.role === "provider" || user.role === "business") && (
                      <DropdownMenuItem onClick={() => router.push("/wallet")}>
                        Wallet
                      </DropdownMenuItem>
                    )}
                    {isAdmin && (
                      <DropdownMenuItem onClick={() => router.push("/admin")} className="gap-2 text-primary font-medium">
                        <ShieldCheck className="h-4 w-4" />
                        Admin Panel
                      </DropdownMenuItem>
                    )}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onClick={() => router.push("/profile")}>
                      Profile Settings
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => startSiteTour(user.role)} className="gap-2">
                      <Compass className="h-4 w-4" />
                      Take a tour
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={handleLogout}
                      className="text-destructive focus:text-destructive"
                    >
                      Log out
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </>
            ) : (
              <>
                <Link href="/login">
                  <Button variant="ghost" size="sm">
                    Log in
                  </Button>
                </Link>
                <Link href="/register">
                  <Button size="sm">Sign up</Button>
                </Link>
              </>
            )}

            {/* Mobile menu toggle */}
            <Button
              variant="ghost"
              size="icon"
              className="md:hidden"
              onClick={() => setMobileOpen((v) => !v)}
            >
              {mobileOpen ? (
                <X className="h-5 w-5" />
              ) : (
                <Menu className="h-5 w-5" />
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* Mobile nav */}
      {mobileOpen && (
        <div className="md:hidden border-t border-border bg-white px-4 py-4 flex flex-col gap-3">
          {navLinks.map((l) => (
            <Link
              key={l.href}
              href={l.href}
              className="text-sm font-medium text-foreground py-1"
              onClick={() => setMobileOpen(false)}
            >
              {l.label}
            </Link>
          ))}
          <Link
            href="/search"
            className="flex items-center gap-2 text-sm font-medium text-muted-foreground py-1"
            onClick={() => setMobileOpen(false)}
          >
            <Search className="h-4 w-4" />
            Search
          </Link>
        </div>
      )}
    </header>
  );
}

import Navbar from "@/components/Navbar";
import EmailVerificationBanner from "@/components/EmailVerificationBanner";
import CompleteProfileBanner from "@/components/CompleteProfileBanner";
import AuthSync from "@/components/AuthSync";
import SiteTour from "@/components/SiteTour";

export default function MainLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <>
      <AuthSync />
      <SiteTour />
      <Navbar />
      <EmailVerificationBanner />
      <CompleteProfileBanner />
      <main className="flex-1">{children}</main>
      <footer className="border-t border-border bg-white mt-auto">
        <div className="mx-auto max-w-6xl px-4 sm:px-6 py-8 flex flex-col sm:flex-row items-center justify-between gap-4 text-sm text-muted-foreground">
          <span>© 2026 MtaaLink. Connecting Kenya, one service at a time.</span>
          <div className="flex gap-6">
            <a href="/about" className="hover:text-foreground transition-colors">About</a>
            <a href="/terms" className="hover:text-foreground transition-colors">Terms</a>
            <a href="/privacy" className="hover:text-foreground transition-colors">Privacy</a>
          </div>
        </div>
      </footer>
    </>
  );
}

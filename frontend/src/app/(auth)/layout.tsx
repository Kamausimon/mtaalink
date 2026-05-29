import Link from "next/link";

export default function AuthLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-background flex flex-col">
      <div className="p-4 sm:p-6">
        <Link href="/" className="text-xl font-bold text-primary tracking-tight">
          Mtaa<span className="text-accent">Link</span>
        </Link>
      </div>
      <div className="flex-1 flex items-center justify-center px-4 py-8">
        {children}
      </div>
    </div>
  );
}

"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type AdminUser, ApiError } from "@/lib/api";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog";
import { Separator } from "@/components/ui/separator";
import { toast } from "sonner";
import { Search, Trash2, CheckCircle2, XCircle, ShieldCheck } from "lucide-react";
import { cn } from "@/lib/utils";

const ROLE_COLORS: Record<string, string> = {
  client: "bg-blue-100 text-blue-700",
  provider: "bg-green-100 text-green-700",
  business: "bg-purple-100 text-purple-700",
};

export default function AdminUsersPage() {
  const { token, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [deleteId, setDeleteId] = useState<number | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [approving, setApproving] = useState<number | null>(null);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    api.admin.users(token)
      .then((r) => setUsers(r.users))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 403) router.replace("/dashboard");
        else toast.error("Failed to load users");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  async function handleDelete() {
    if (!deleteId || !token) return;
    setDeleting(true);
    try {
      await api.admin.deleteUser(deleteId, token);
      setUsers((u) => u.filter((x) => x.id !== deleteId));
      toast.success("User deleted");
    } catch {
      toast.error("Failed to delete user");
    } finally {
      setDeleting(false);
      setDeleteId(null);
    }
  }

  async function handleApprove(user: AdminUser) {
    if (!token) return;
    const entityType = user.provider_id ? "provider" : "business";
    const entityId = user.provider_id ?? user.business_id;
    if (!entityId) return;

    setApproving(user.id);
    try {
      await api.admin.approve(entityType, entityId, true, token);
      setUsers((prev) =>
        prev.map((u) =>
          u.id === user.id
            ? {
                ...u,
                provider_approved: u.provider_id ? true : u.provider_approved,
                business_verified: u.business_id ? true : u.business_verified,
              }
            : u,
        ),
      );
      toast.success(`${entityType === "provider" ? "Provider approved" : "Business verified"}`);
    } catch {
      toast.error("Failed to approve");
    } finally {
      setApproving(null);
    }
  }

  const filtered = users.filter(
    (u) =>
      u.username.toLowerCase().includes(query.toLowerCase()) ||
      u.email.toLowerCase().includes(query.toLowerCase()),
  );

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-4 max-w-4xl">
        <Skeleton className="h-8 w-32" />
        <Skeleton className="h-10 w-full" />
        {Array.from({ length: 8 }).map((_, i) => <Skeleton key={i} className="h-14 rounded-lg" />)}
      </div>
    );
  }

  const userToDelete = users.find((u) => u.id === deleteId);

  return (
    <div className="space-y-5 max-w-4xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Users</h1>
        <p className="text-sm text-muted-foreground mt-1">{users.length} registered users</p>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="Search by username or email…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="pl-9"
        />
      </div>

      <div className="rounded-xl border border-border overflow-hidden bg-white">
        <div className="grid grid-cols-[1fr_2fr_80px_120px_auto] items-center gap-3 px-4 py-3 bg-muted/40 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          <span>Username</span>
          <span>Email</span>
          <span>Role</span>
          <span>Status</span>
          <span />
        </div>
        <Separator />
        {filtered.length === 0 ? (
          <p className="text-sm text-muted-foreground px-4 py-8 text-center">No users found.</p>
        ) : (
          filtered.map((u, i) => {
            const isProvider = u.role === "provider";
            const isBusiness = u.role === "business";
            const isApproved = isProvider ? u.provider_approved : isBusiness ? u.business_verified : null;
            const needsApproval = (isProvider || isBusiness) && isApproved === false;
            const isApprovingThis = approving === u.id;

            return (
              <div key={u.id}>
                {i > 0 && <Separator />}
                <div className={cn(
                  "grid grid-cols-[1fr_2fr_80px_120px_auto] items-center gap-3 px-4 py-3",
                  needsApproval && "bg-amber-50/50",
                )}>
                  <span className="text-sm font-medium text-foreground truncate">{u.username}</span>
                  <span className="text-sm text-muted-foreground truncate">{u.email}</span>
                  <span
                    className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-semibold ${
                      ROLE_COLORS[u.role ?? ""] ?? "bg-gray-100 text-gray-700"
                    }`}
                  >
                    {u.role ?? "—"}
                  </span>

                  {/* Approval status */}
                  <div className="flex items-center gap-1.5">
                    {isApproved === true && (
                      <span className="inline-flex items-center gap-1 text-xs text-green-700 bg-green-50 border border-green-200 rounded-full px-2 py-0.5 font-medium">
                        <CheckCircle2 className="h-3 w-3" />
                        {isProvider ? "Approved" : "Verified"}
                      </span>
                    )}
                    {isApproved === false && (
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-7 text-xs border-amber-300 text-amber-700 hover:bg-amber-50 gap-1"
                        onClick={() => handleApprove(u)}
                        disabled={isApprovingThis}
                      >
                        <ShieldCheck className="h-3 w-3" />
                        {isApprovingThis ? "…" : (isProvider ? "Approve" : "Verify")}
                      </Button>
                    )}
                    {isApproved === null && (
                      <span className="text-xs text-muted-foreground">—</span>
                    )}
                  </div>

                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-muted-foreground hover:text-destructive"
                    onClick={() => setDeleteId(u.id)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            );
          })
        )}
      </div>

      <Dialog open={deleteId !== null} onOpenChange={(o) => !o && setDeleteId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete user?</DialogTitle>
            <DialogDescription>
              This will permanently delete <strong>{userToDelete?.username}</strong> ({userToDelete?.email}) and all their associated data. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteId(null)} disabled={deleting}>
              Cancel
            </Button>
            <Button
              onClick={handleDelete}
              disabled={deleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleting ? "Deleting…" : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

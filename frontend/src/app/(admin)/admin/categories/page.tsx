"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Category, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Trash2, Plus, FolderTree } from "lucide-react";

export default function AdminCategoriesPage() {
  const { token, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [parentId, setParentId] = useState<string>("none");
  const [adding, setAdding] = useState(false);
  const [deleteId, setDeleteId] = useState<number | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    api.admin.categories(token)
      .then((r) => setCategories(r.categories))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 403) router.replace("/dashboard");
        else toast.error("Failed to load categories");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  async function handleAdd() {
    if (!name.trim() || !token) return;
    setAdding(true);
    try {
      await api.admin.createCategory(name.trim(), parentId === "none" ? null : Number(parentId), token);
      const r = await api.admin.categories(token);
      setCategories(r.categories);
      setName("");
      setParentId("none");
      toast.success("Category created");
    } catch {
      toast.error("Failed to create category");
    } finally {
      setAdding(false);
    }
  }

  async function handleDelete() {
    if (!deleteId || !token) return;
    setDeleting(true);
    try {
      await api.admin.deleteCategory(deleteId, token);
      setCategories((c) => c.filter((x) => x.id !== deleteId));
      toast.success("Category deleted");
    } catch {
      toast.error("Failed to delete category");
    } finally {
      setDeleting(false);
      setDeleteId(null);
    }
  }

  const parentCategories = categories.filter((c) => !c.parent_id);
  const catToDelete = categories.find((c) => c.id === deleteId);

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-4 max-w-3xl">
        <Skeleton className="h-8 w-40" />
        <Skeleton className="h-32 rounded-xl" />
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Categories</h1>
        <p className="text-sm text-muted-foreground mt-1">{categories.length} categories</p>
      </div>

      {/* Add category form */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base flex items-center gap-2">
            <Plus className="h-4 w-4" />
            Add category
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="cat-name">Name</Label>
            <Input
              id="cat-name"
              placeholder="Category name…"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAdd()}
            />
          </div>
          <div className="space-y-1.5">
            <Label>Parent category (optional)</Label>
            <Select value={parentId} onValueChange={(v) => setParentId(v ?? "none")}>
              <SelectTrigger>
                <SelectValue placeholder="Top-level category" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">— Top-level —</SelectItem>
                {parentCategories.map((c) => (
                  <SelectItem key={c.id} value={String(c.id)}>
                    {c.category_name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Button onClick={handleAdd} disabled={adding || !name.trim()}>
            {adding ? "Adding…" : "Add category"}
          </Button>
        </CardContent>
      </Card>

      {/* Category tree */}
      <div className="rounded-xl border border-border overflow-hidden bg-white">
        <div className="flex items-center gap-2 px-5 py-3 bg-muted/40 border-b border-border">
          <FolderTree className="h-4 w-4 text-muted-foreground" />
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Category tree</span>
        </div>
        {categories.length === 0 ? (
          <p className="text-sm text-muted-foreground px-5 py-8 text-center">No categories yet.</p>
        ) : (
          parentCategories.map((parent, pi) => {
            const children = categories.filter((c) => c.parent_id === parent.id);
            return (
              <div key={parent.id}>
                {pi > 0 && <Separator />}
                {/* Parent row */}
                <div className="flex items-center justify-between px-5 py-3 bg-muted/20">
                  <span className="text-sm font-semibold text-foreground">{parent.category_name}</span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-muted-foreground hover:text-destructive"
                    onClick={() => setDeleteId(parent.id)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
                {/* Children */}
                {children.map((child, ci) => (
                  <div key={child.id}>
                    <Separator />
                    <div className="flex items-center justify-between px-5 py-2.5 pl-10">
                      <span className="text-sm text-muted-foreground">{child.category_name}</span>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 text-muted-foreground hover:text-destructive"
                        onClick={() => setDeleteId(child.id)}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            );
          })
        )}
        {/* Orphan subcategories (parent deleted) */}
        {categories
          .filter((c) => c.parent_id && !parentCategories.find((p) => p.id === c.parent_id))
          .map((c, i) => (
            <div key={c.id}>
              <Separator />
              <div className="flex items-center justify-between px-5 py-3">
                <div>
                  <span className="text-sm font-medium text-foreground">{c.category_name}</span>
                  <span className="text-xs text-muted-foreground ml-2">(under: {c.parent_name ?? `#${c.parent_id}`})</span>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-muted-foreground hover:text-destructive"
                  onClick={() => setDeleteId(c.id)}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
          ))}
      </div>

      <Dialog open={deleteId !== null} onOpenChange={(o) => !o && setDeleteId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete category?</DialogTitle>
            <DialogDescription>
              Delete <strong>{catToDelete?.category_name}</strong>? Subcategories under this parent may be affected.
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

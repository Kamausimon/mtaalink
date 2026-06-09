"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type ManagedService, type CreateServiceInput } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription,
} from "@/components/ui/dialog";
import { Plus, Pencil, Trash2, Clock, BadgeDollarSign } from "lucide-react";
import { toast } from "sonner";

type ServiceForm = {
  title: string;
  description: string;
  price: string;
  duration: string;
  is_active: boolean;
};

const EMPTY_FORM: ServiceForm = { title: "", description: "", price: "", duration: "", is_active: true };

export default function ServicesPage() {
  const { token, user, isAuthenticated, _hasHydrated } = useAuthStore();
  const router = useRouter();

  const [services, setServices] = useState<ManagedService[]>([]);
  const [loading, setLoading] = useState(true);
  const [targetId, setTargetId] = useState<number | null>(null);
  const [targetType, setTargetType] = useState<"provider" | "business">("provider");

  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<ManagedService | null>(null);
  const [form, setForm] = useState<ServiceForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);

  const [deleteId, setDeleteId] = useState<number | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.replace("/login"); return; }
    if (user?.role === "client") { router.replace("/dashboard"); return; }

    api.dashboard.get(token!).then((d) => {
      const type = d.role === "business" ? "business" : "provider";
      const id = type === "business" ? d.business_id : d.provider_id;
      if (!id) { router.replace("/dashboard"); return; }
      setTargetType(type);
      setTargetId(id);
      return api.services.list(type, id, token!);
    }).then((res) => {
      if (res) setServices(res.services);
    }).catch(() => toast.error("Could not load services"))
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated, isAuthenticated]);

  function openAdd() {
    setEditing(null);
    setForm(EMPTY_FORM);
    setDialogOpen(true);
  }

  function openEdit(svc: ManagedService) {
    setEditing(svc);
    setForm({
      title: svc.title,
      description: svc.description ?? "",
      price: svc.price ? String(Number(svc.price)) : "",
      duration: svc.duration ? String(svc.duration) : "",
      is_active: svc.is_active,
    });
    setDialogOpen(true);
  }

  function closeDialog() {
    setDialogOpen(false);
    setEditing(null);
    setForm(EMPTY_FORM);
  }

  async function save() {
    if (!form.title.trim()) { toast.error("Title is required"); return; }
    if (!targetId) return;
    setSaving(true);
    try {
      if (editing) {
        await api.services.update({
          service_id: editing.id,
          target_id: editing.target_id,
          target_type: editing.target_type,
          title: form.title.trim(),
          description: form.description.trim() || undefined,
          price: form.price ? Number(form.price) : undefined,
          duration: form.duration ? Number(form.duration) : undefined,
          is_active: form.is_active,
        }, token!);
        setServices((prev) => prev.map((s) => s.id === editing.id
          ? { ...s, title: form.title.trim(), description: form.description.trim(),
              price: form.price, duration: Number(form.duration), is_active: form.is_active }
          : s));
        toast.success("Service updated");
      } else {
        const data: CreateServiceInput = {
          target_id: targetId,
          target_type: targetType,
          title: form.title.trim(),
          description: form.description.trim(),
          price: form.price ? Number(form.price) : 0,
          duration: form.duration ? Number(form.duration) : 60,
          is_active: form.is_active,
        };
        const res = await api.services.create(data, token!);
        setServices((prev) => [...prev, {
          id: res.service_id, target_id: targetId, target_type: targetType,
          title: data.title, description: data.description, price: String(data.price),
          duration: data.duration, is_active: data.is_active,
        }]);
        toast.success("Service added");
      }
      closeDialog();
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Could not save service");
    } finally {
      setSaving(false);
    }
  }

  async function confirmDelete() {
    if (!deleteId) return;
    setDeleting(true);
    try {
      await api.services.delete(deleteId, token!);
      setServices((prev) => prev.filter((s) => s.id !== deleteId));
      toast.success("Service deleted");
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Could not delete service");
    } finally {
      setDeleting(false);
      setDeleteId(null);
    }
  }

  async function toggleActive(svc: ManagedService) {
    try {
      await api.services.update({
        service_id: svc.id,
        target_id: svc.target_id,
        target_type: svc.target_type,
        is_active: !svc.is_active,
      }, token!);
      setServices((prev) => prev.map((s) => s.id === svc.id ? { ...s, is_active: !s.is_active } : s));
    } catch {
      toast.error("Could not update service");
    }
  }

  if (!_hasHydrated || loading) {
    return (
      <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-4">
        <Skeleton className="h-10 w-40" />
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  return (
    <>
      <div className="mx-auto max-w-2xl px-4 sm:px-6 py-8 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-foreground">My Services</h1>
            <p className="text-sm text-muted-foreground mt-0.5">
              {services.length} service{services.length !== 1 ? "s" : ""} listed
            </p>
          </div>
          <Button onClick={openAdd} className="gap-2">
            <Plus className="h-4 w-4" />
            Add service
          </Button>
        </div>

        <Card className="border border-border">
          <CardHeader className="pb-3">
            <CardTitle className="text-base">Services</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            {services.length === 0 ? (
              <div className="py-12 text-center">
                <BadgeDollarSign className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
                <p className="font-medium text-foreground">No services yet</p>
                <p className="text-sm text-muted-foreground mt-1 mb-4">
                  Add your first service so clients can book you.
                </p>
                <Button onClick={openAdd} className="gap-2">
                  <Plus className="h-4 w-4" />Add service
                </Button>
              </div>
            ) : (
              services.map((svc, i) => (
                <div key={svc.id}>
                  {i > 0 && <Separator />}
                  <div className="flex items-start gap-4 px-5 py-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <p className="text-sm font-semibold text-foreground">{svc.title}</p>
                        {!svc.is_active && (
                          <span className="text-xs bg-muted text-muted-foreground px-2 py-0.5 rounded-full">
                            Inactive
                          </span>
                        )}
                      </div>
                      {svc.description && (
                        <p className="text-sm text-muted-foreground mt-0.5 line-clamp-2">{svc.description}</p>
                      )}
                      <div className="flex items-center gap-4 mt-1.5 text-xs text-muted-foreground">
                        {svc.price && (
                          <span className="font-medium text-foreground">
                            KES {Number(svc.price).toLocaleString()}
                          </span>
                        )}
                        {svc.duration && (
                          <span className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />{svc.duration} min
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <Switch
                        checked={svc.is_active}
                        onCheckedChange={() => toggleActive(svc)}
                        title={svc.is_active ? "Deactivate" : "Activate"}
                      />
                      <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => openEdit(svc)}>
                        <Pencil className="h-3.5 w-3.5" />
                      </Button>
                      <Button
                        variant="ghost" size="icon"
                        className="h-8 w-8 text-muted-foreground hover:text-destructive"
                        onClick={() => setDeleteId(svc.id)}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>

      {/* Add / Edit dialog */}
      <Dialog open={dialogOpen} onOpenChange={(v) => { if (!v) closeDialog(); }}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit service" : "Add service"}</DialogTitle>
            <DialogDescription>
              {editing ? "Update the details for this service." : "Add a new service clients can book."}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 pt-1">
            <div className="space-y-1.5">
              <Label>Service title <span className="text-destructive">*</span></Label>
              <Input
                placeholder="e.g. Full house wiring"
                value={form.title}
                onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Description</Label>
              <Textarea
                placeholder="Describe what's included…"
                rows={3}
                className="resize-none"
                value={form.description}
                onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label>Price (KES)</Label>
                <Input
                  type="number"
                  min="0"
                  placeholder="e.g. 2500"
                  value={form.price}
                  onChange={(e) => setForm((f) => ({ ...f, price: e.target.value }))}
                />
              </div>
              <div className="space-y-1.5">
                <Label>Duration (minutes)</Label>
                <Input
                  type="number"
                  min="1"
                  placeholder="e.g. 120"
                  value={form.duration}
                  onChange={(e) => setForm((f) => ({ ...f, duration: e.target.value }))}
                />
              </div>
            </div>
            <div className="flex items-center justify-between py-1">
              <div>
                <p className="text-sm font-medium text-foreground">Active</p>
                <p className="text-xs text-muted-foreground">Clients can book this service</p>
              </div>
              <Switch
                checked={form.is_active}
                onCheckedChange={(v) => setForm((f) => ({ ...f, is_active: v }))}
              />
            </div>
            <div className="flex gap-2 pt-1">
              <Button variant="outline" className="flex-1" onClick={closeDialog} disabled={saving}>
                Cancel
              </Button>
              <Button className="flex-1" onClick={save} disabled={saving}>
                {saving ? "Saving…" : editing ? "Save changes" : "Add service"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Delete confirmation */}
      <Dialog open={deleteId !== null} onOpenChange={(v) => { if (!v) setDeleteId(null); }}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Delete service?</DialogTitle>
            <DialogDescription>
              This will permanently remove the service. Existing bookings are not affected.
            </DialogDescription>
          </DialogHeader>
          <div className="flex gap-2 pt-2">
            <Button variant="outline" className="flex-1" onClick={() => setDeleteId(null)} disabled={deleting}>
              Cancel
            </Button>
            <Button
              className="flex-1 bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={confirmDelete}
              disabled={deleting}
            >
              {deleting ? "Deleting…" : "Delete"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

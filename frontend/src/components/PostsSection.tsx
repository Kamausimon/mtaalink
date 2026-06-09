"use client";

import { useEffect, useState, useRef } from "react";
import { api, type Post, type PostComment } from "@/lib/api";
import { useAuthStore } from "@/store/auth";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Heart, MessageCircle, Trash2, Plus, ImagePlus, X, ChevronLeft, ChevronRight, Loader2 } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

const CLOUD_NAME = process.env.NEXT_PUBLIC_CLOUDINARY_CLOUD_NAME!;
const UPLOAD_PRESET = process.env.NEXT_PUBLIC_CLOUDINARY_UPLOAD_PRESET!;

async function uploadToCloudinary(file: File): Promise<string> {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("upload_preset", UPLOAD_PRESET);
  const res = await fetch(`https://api.cloudinary.com/v1_1/${CLOUD_NAME}/image/upload`, {
    method: "POST",
    body: formData,
  });
  if (!res.ok) throw new Error("Image upload failed");
  const data = await res.json();
  return data.secure_url as string;
}

// ── Image carousel ────────────────────────────────────────────────────────────

function ImageCarousel({ urls }: { urls: string[] }) {
  const [idx, setIdx] = useState(0);
  if (urls.length === 0) return null;
  return (
    <div className="relative w-full aspect-4/3 bg-muted rounded-t-lg overflow-hidden">
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src={urls[idx]} alt={`Image ${idx + 1}`} className="w-full h-full object-cover" />
      {urls.length > 1 && (
        <>
          <button
            type="button"
            onClick={() => setIdx((i) => (i - 1 + urls.length) % urls.length)}
            className="absolute left-2 top-1/2 -translate-y-1/2 bg-black/40 hover:bg-black/60 text-white rounded-full p-1 transition-colors"
          >
            <ChevronLeft className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={() => setIdx((i) => (i + 1) % urls.length)}
            className="absolute right-2 top-1/2 -translate-y-1/2 bg-black/40 hover:bg-black/60 text-white rounded-full p-1 transition-colors"
          >
            <ChevronRight className="h-4 w-4" />
          </button>
          <div className="absolute bottom-2 left-1/2 -translate-x-1/2 flex gap-1">
            {urls.map((_, i) => (
              <button key={i} type="button" onClick={() => setIdx(i)}
                className={cn("h-1.5 w-1.5 rounded-full transition-colors", i === idx ? "bg-white" : "bg-white/40")} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

// ── Comment thread ────────────────────────────────────────────────────────────

function CommentThread({ postId, onCountChange }: { postId: number; onCountChange: (n: number) => void }) {
  const { token, user, isAuthenticated, _hasHydrated } = useAuthStore();
  const [comments, setComments] = useState<PostComment[]>([]);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);

  useEffect(() => {
    api.posts.comments(postId).then((r) => {
      setComments(r.comments);
      onCountChange(r.comments.length);
    }).catch(() => {});
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [postId]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!input.trim()) return;
    if (!token || !isAuthenticated) {
      toast.error("Please sign in to comment");
      return;
    }
    setSending(true);
    try {
      await api.posts.addComment(postId, input.trim(), token);
      setComments((prev) => {
        const updated = [...prev, {
          id: Date.now(), user_id: user!.id, username: user!.username,
          comment: input.trim(), created_at: new Date().toISOString(),
        }];
        onCountChange(updated.length);
        return updated;
      });
      setInput("");
    } catch (err: unknown) {
      const status = err instanceof Error && "status" in err ? (err as { status: number }).status : 0;
      if (status === 401) {
        toast.error("Session expired — please log in again");
      } else {
        toast.error("Could not post comment");
      }
    }
    finally { setSending(false); }
  }

  async function remove(commentId: number) {
    if (!token) return;
    try {
      await api.posts.deleteComment(postId, commentId, token);
      setComments((prev) => {
        const updated = prev.filter((c) => c.id !== commentId);
        onCountChange(updated.length);
        return updated;
      });
    } catch { toast.error("Could not delete comment"); }
  }

  return (
    <div className="mt-3 space-y-2">
      {comments.map((c) => (
        <div key={c.id} className="flex items-start gap-2">
          <Avatar className="h-6 w-6 shrink-0">
            <AvatarFallback className="text-xs bg-primary/10 text-primary">
              {c.username.slice(0, 2).toUpperCase()}
            </AvatarFallback>
          </Avatar>
          <div className="flex-1 bg-muted rounded-xl px-3 py-1.5 text-sm">
            <span className="font-medium text-foreground mr-1.5">{c.username}</span>
            <span className="text-foreground/80">{c.comment}</span>
          </div>
          {user?.id === c.user_id && (
            <button type="button" onClick={() => remove(c.id)} className="text-muted-foreground hover:text-destructive mt-1">
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      ))}
      {_hasHydrated && isAuthenticated && (
        <form onSubmit={submit} className="flex gap-2 mt-1">
          <Input value={input} onChange={(e) => setInput(e.target.value)}
            placeholder="Add a comment…" className="h-8 text-sm" disabled={sending} />
          <Button type="submit" size="sm" className="h-8 shrink-0" disabled={sending || !input.trim()}>
            {sending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "Post"}
          </Button>
        </form>
      )}
    </div>
  );
}

// ── Post card ─────────────────────────────────────────────────────────────────

function PostCard({ post, isOwner, onDeleted }: { post: Post; isOwner: boolean; onDeleted: (id: number) => void }) {
  const { token, isAuthenticated } = useAuthStore();
  const [liked, setLiked] = useState(false);
  const [likeCount, setLikeCount] = useState(post.like_count);
  const [commentCount, setCommentCount] = useState(post.comment_count ?? 0);
  const [showComments, setShowComments] = useState(false);
  const [deleting, setDeleting] = useState(false);

  async function toggleLike() {
    if (!isAuthenticated || !token) { toast.error("Sign in to like posts"); return; }
    try {
      if (liked) {
        await api.posts.unlike(post.id, token);
        setLikeCount((n) => n - 1);
      } else {
        await api.posts.like(post.id, token);
        setLikeCount((n) => n + 1);
      }
      setLiked((v) => !v);
    } catch { toast.error("Action failed"); }
  }

  async function deletePost() {
    if (!token) return;
    setDeleting(true);
    try {
      await api.posts.delete(post.id, token);
      onDeleted(post.id);
      toast.success("Post deleted");
    } catch { toast.error("Could not delete post"); setDeleting(false); }
  }

  return (
    <Card className="border border-border overflow-hidden">
      <ImageCarousel urls={post.image_urls} />
      <CardContent className="p-4 space-y-2">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <p className="font-semibold text-sm text-foreground">{post.title}</p>
            <p className="text-sm text-muted-foreground mt-0.5 leading-relaxed whitespace-pre-wrap">{post.content}</p>
          </div>
          {isOwner && (
            <button type="button" disabled={deleting} onClick={deletePost}
              className="text-muted-foreground hover:text-destructive shrink-0 mt-0.5 disabled:opacity-50">
              <Trash2 className="h-4 w-4" />
            </button>
          )}
        </div>

        <div className="flex items-center gap-4 pt-1">
          <button type="button" onClick={toggleLike}
            className={cn("flex items-center gap-1.5 text-sm transition-colors", liked ? "text-rose-500" : "text-muted-foreground hover:text-rose-500")}>
            <Heart className={cn("h-4 w-4", liked && "fill-rose-500")} />
            <span>{likeCount}</span>
          </button>
          <button type="button" onClick={() => setShowComments((v) => !v)}
            className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors">
            <MessageCircle className="h-4 w-4" />
            <span>{commentCount > 0 ? `${commentCount} comment${commentCount === 1 ? "" : "s"}` : "Comment"}</span>
          </button>
          <span className="text-xs text-muted-foreground ml-auto">
            {format(new Date(post.created_at), "d MMM yyyy")}
          </span>
        </div>

        {showComments && (
          <CommentThread
            postId={post.id}
            onCountChange={(n) => setCommentCount(n)}
          />
        )}
      </CardContent>
    </Card>
  );
}

// ── Create post dialog ────────────────────────────────────────────────────────

type CreatePostDialogProps = {
  open: boolean;
  onClose: () => void;
  providerId?: number;
  businessId?: number;
  onCreated: (post: Post) => void;
};

function CreatePostDialog({ open, onClose, providerId, businessId, onCreated }: CreatePostDialogProps) {
  const { token } = useAuthStore();
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [previews, setPreviews] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  function pickFiles(e: React.ChangeEvent<HTMLInputElement>) {
    const picked = Array.from(e.target.files ?? []).slice(0, 5 - files.length);
    if (picked.length === 0) return;
    setFiles((prev) => [...prev, ...picked].slice(0, 5));
    picked.forEach((f) => {
      const reader = new FileReader();
      reader.onload = (ev) => setPreviews((prev) => [...prev, ev.target!.result as string].slice(0, 5));
      reader.readAsDataURL(f);
    });
    e.target.value = "";
  }

  function removeFile(i: number) {
    setFiles((prev) => prev.filter((_, idx) => idx !== i));
    setPreviews((prev) => prev.filter((_, idx) => idx !== i));
  }

  function reset() {
    setTitle(""); setContent(""); setFiles([]); setPreviews([]);
  }

  async function submit() {
    if (!title.trim()) { toast.error("Title is required"); return; }
    if (!content.trim()) { toast.error("Caption is required"); return; }
    setSaving(true);
    try {
      // 1. Upload images to Cloudinary
      const imageUrls = await Promise.all(files.map(uploadToCloudinary));

      // 2. Create the post
      const { post_id } = await api.posts.create(
        { title: title.trim(), content: content.trim(), provider_id: providerId, business_id: businessId },
        token!,
      );

      // 3. Attach the image URLs
      if (imageUrls.length > 0) {
        await api.posts.update(post_id, { attachments: imageUrls }, token!);
      }

      const newPost: Post = {
        id: post_id,
        title: title.trim(),
        content: content.trim(),
        provider_id: providerId,
        business_id: businessId,
        image_urls: imageUrls,
        like_count: 0,
        comment_count: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      toast.success("Post published!");
      onCreated(newPost);
      reset();
      onClose();
    } catch (err: unknown) {
      toast.error(err instanceof Error ? err.message : "Could not publish post");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) { reset(); onClose(); } }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>New post</DialogTitle>
          <DialogDescription>Share your work — add photos and a caption.</DialogDescription>
        </DialogHeader>
        <div className="space-y-4 pt-1">
          <div className="space-y-1.5">
            <Label>Title</Label>
            <Input placeholder="e.g. Bathroom renovation completed" value={title} onChange={(e) => setTitle(e.target.value)} />
          </div>
          <div className="space-y-1.5">
            <Label>Caption</Label>
            <Textarea placeholder="Describe the work, materials used, or anything clients should know…"
              rows={3} value={content} onChange={(e) => setContent(e.target.value)} className="resize-none" />
          </div>

          {/* Image picker */}
          <div className="space-y-2">
            <Label>Photos <span className="text-muted-foreground text-xs">(up to 5)</span></Label>
            {previews.length > 0 && (
              <div className="grid grid-cols-3 gap-2">
                {previews.map((src, i) => (
                  <div key={i} className="relative aspect-square rounded-lg overflow-hidden border border-border">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img src={src} alt="" className="w-full h-full object-cover" />
                    <button type="button" onClick={() => removeFile(i)}
                      className="absolute top-1 right-1 bg-black/50 hover:bg-black/70 text-white rounded-full p-0.5 transition-colors">
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                ))}
                {previews.length < 5 && (
                  <button type="button" onClick={() => fileRef.current?.click()}
                    className="aspect-square rounded-lg border-2 border-dashed border-border hover:border-primary flex items-center justify-center transition-colors">
                    <Plus className="h-5 w-5 text-muted-foreground" />
                  </button>
                )}
              </div>
            )}
            {previews.length === 0 && (
              <button type="button" onClick={() => fileRef.current?.click()}
                className="w-full border-2 border-dashed border-border hover:border-primary rounded-lg p-8 flex flex-col items-center gap-2 transition-colors">
                <ImagePlus className="h-8 w-8 text-muted-foreground" />
                <span className="text-sm text-muted-foreground">Click to add photos</span>
              </button>
            )}
            <input ref={fileRef} type="file" accept="image/*" multiple className="hidden" onChange={pickFiles} />
          </div>

          <div className="flex gap-2 justify-end pt-1">
            <Button variant="outline" onClick={() => { reset(); onClose(); }} disabled={saving}>Cancel</Button>
            <Button onClick={submit} disabled={saving} className="gap-2">
              {saving && <Loader2 className="h-4 w-4 animate-spin" />}
              {saving ? "Publishing…" : "Publish"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ── Main section ──────────────────────────────────────────────────────────────

type PostsSectionProps = {
  targetType: "provider" | "business";
  targetId: number;
  isOwner: boolean;
};

export default function PostsSection({ targetType, targetId, isOwner }: PostsSectionProps) {
  const [posts, setPosts] = useState<Post[]>([]);
  const [loading, setLoading] = useState(true);
  const [createOpen, setCreateOpen] = useState(false);

  useEffect(() => {
    const req = targetType === "provider"
      ? api.posts.byProvider(targetId)
      : api.posts.byBusiness(targetId);

    req
      .then((r) => setPosts(r.posts))
      .catch(() => toast.error("Could not load portfolio"))
      .finally(() => setLoading(false));
  }, [targetType, targetId]);

  return (
    <Card className="border border-border">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">
            Portfolio
            {posts.length > 0 && (
              <span className="ml-1.5 text-sm font-normal text-muted-foreground">({posts.length})</span>
            )}
          </CardTitle>
          {isOwner && (
            <Button size="sm" onClick={() => setCreateOpen(true)} className="gap-1.5">
              <Plus className="h-4 w-4" />New post
            </Button>
          )}
        </div>
      </CardHeader>

      <CardContent>
        {loading ? (
          <div className="grid sm:grid-cols-2 gap-4">
            {Array.from({ length: 2 }).map((_, i) => (
              <div key={i} className="rounded-xl bg-muted animate-pulse aspect-4/3" />
            ))}
          </div>
        ) : posts.length === 0 ? (
          <div className="text-center py-8 border-2 border-dashed border-border rounded-xl">
            <ImagePlus className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">
              {isOwner ? "Share your work — create your first post." : "No posts yet."}
            </p>
            {isOwner && (
              <Button size="sm" className="mt-3" onClick={() => setCreateOpen(true)}>
                Create first post
              </Button>
            )}
          </div>
        ) : (
          <div className="grid sm:grid-cols-2 gap-4">
            {posts.map((post) => (
              <PostCard
                key={post.id}
                post={post}
                isOwner={isOwner}
                onDeleted={(id) => setPosts((prev) => prev.filter((p) => p.id !== id))}
              />
            ))}
          </div>
        )}
      </CardContent>

      <CreatePostDialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        providerId={targetType === "provider" ? targetId : undefined}
        businessId={targetType === "business" ? targetId : undefined}
        onCreated={(post) => setPosts((prev) => [post, ...prev])}
      />
    </Card>
  );
}

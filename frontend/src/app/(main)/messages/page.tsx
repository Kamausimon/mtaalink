"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Conversation, type Message } from "@/lib/api";
import { useWebSocket } from "@/hooks/useWebSocket";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Send, MessageCircle, ImagePlus, X } from "lucide-react";
import { format, isToday, isThisYear } from "date-fns";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

function formatConvTime(iso: string) {
  const d = new Date(iso);
  if (isToday(d)) return format(d, "h:mm a");
  if (isThisYear(d)) return format(d, "d MMM");
  return format(d, "d MMM yyyy");
}

function isImageUrl(content: string) {
  return content.startsWith("https://res.cloudinary.com/") ||
    (content.startsWith("/uploads/") && /\.(jpg|jpeg|png|webp|heic|gif)$/i.test(content));
}

type WsMessage = {
  id: number;
  sender_id: number;
  content: string;
  target_type: string;
  target_id: number;
  created_at: string;
};

type PendingImage = { file: File; preview: string };

export default function MessagesPage() {
  const { token, user, isAuthenticated, _hasHydrated } = useAuthStore();
  const router = useRouter();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [selected, setSelected] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [msgInput, setMsgInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [pendingImage, setPendingImage] = useState<PendingImage | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const selectedRef = useRef<Conversation | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    selectedRef.current = selected;
  }, [selected]);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!isAuthenticated) { router.push("/login"); return; }
    api.messages
      .conversations(token!)
      .then((r) => setConversations(r.conversations))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [_hasHydrated, isAuthenticated, token, router]);

  useEffect(() => {
    if (!selected) return;
    api.messages
      .get(token!, {
        other_user_id: selected.other_user_id,
        target_type: selected.target_type,
        target_id: selected.target_id,
      })
      .then((r) => {
        setMessages(r.messages);
        const unreadIds = r.messages
          .filter((m) => !m.is_read && m.sender_id !== user?.id)
          .map((m) => m.id);
        if (unreadIds.length > 0) {
          api.messages.markRead(unreadIds, token!).catch(() => {});
        }
      })
      .catch(() => {});
  }, [selected, token, user?.id]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useWebSocket(token ?? null, {
    new_message: (raw) => {
      const msg = raw as WsMessage;
      const cur = selectedRef.current;
      const isCurrentThread =
        cur &&
        cur.other_user_id === msg.sender_id &&
        cur.target_type === msg.target_type &&
        cur.target_id === msg.target_id;

      if (isCurrentThread) {
        setMessages((prev) => [
          ...prev,
          {
            id: msg.id,
            sender_id: msg.sender_id,
            receiver_id: user!.id,
            content: msg.content,
            created_at: msg.created_at,
            is_read: true,
          },
        ]);
        api.messages.markRead([msg.id], token!).catch(() => {});
      }

      setConversations((prev) => {
        const exists = prev.find(
          (c) =>
            c.other_user_id === msg.sender_id &&
            c.target_type === msg.target_type &&
            c.target_id === msg.target_id,
        );
        if (exists) {
          return prev.map((c) =>
            c.other_user_id === msg.sender_id &&
            c.target_type === msg.target_type &&
            c.target_id === msg.target_id
              ? {
                  ...c,
                  last_message: msg.content,
                  last_message_at: msg.created_at,
                  unread_count: isCurrentThread ? 0 : c.unread_count + 1,
                }
              : c,
          );
        }
        api.messages.conversations(token!).then((r) => setConversations(r.conversations)).catch(() => {});
        return prev;
      });
    },
  });

  function selectConversation(conv: Conversation) {
    setSelected(conv);
    setConversations((prev) =>
      prev.map((c) =>
        c.other_user_id === conv.other_user_id &&
        c.target_type === conv.target_type &&
        c.target_id === conv.target_id
          ? { ...c, unread_count: 0 }
          : c,
      ),
    );
  }

  function onImagePick(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/")) { toast.error("Please select an image file"); return; }
    const preview = URL.createObjectURL(file);
    setPendingImage({ file, preview });
    // Reset input so the same file can be picked again
    e.target.value = "";
  }

  function removePendingImage() {
    if (pendingImage) URL.revokeObjectURL(pendingImage.preview);
    setPendingImage(null);
  }

  function appendOptimistic(content: string) {
    const optimistic: Message = {
      id: Date.now() + Math.random(),
      sender_id: user!.id,
      receiver_id: selected!.other_user_id,
      content,
      created_at: new Date().toISOString(),
      is_read: false,
    };
    setMessages((prev) => [...prev, optimistic]);
    setConversations((prev) =>
      prev.map((c) =>
        c.other_user_id === selected!.other_user_id &&
        c.target_type === selected!.target_type &&
        c.target_id === selected!.target_id
          ? { ...c, last_message: content, last_message_at: optimistic.created_at }
          : c,
      ),
    );
    return optimistic.id;
  }

  async function sendMessage(e: React.FormEvent) {
    e.preventDefault();
    const text = msgInput.trim();
    if ((!text && !pendingImage) || !selected || sending) return;

    setSending(true);
    const captionText = text;
    setMsgInput("");

    try {
      // If there's an image, upload it first then send as a message
      if (pendingImage) {
        const optimId = appendOptimistic(pendingImage.preview); // temporary local preview
        removePendingImage();
        try {
          const { url } = await api.messages.uploadAttachment(pendingImage?.file ?? ({} as File));
          // Replace temporary optimistic with real URL
          setMessages((prev) =>
            prev.map((m) =>
              m.id === optimId ? { ...m, content: url } : m,
            ),
          );
          await api.messages.send(
            { receiver_id: selected.other_user_id, content: url, target_type: selected.target_type, target_id: selected.target_id },
            token!,
          );
        } catch {
          setMessages((prev) => prev.filter((m) => m.id !== optimId));
          toast.error("Failed to send image");
        }
      }

      // Send text message if any
      if (captionText) {
        const optimId = appendOptimistic(captionText);
        try {
          await api.messages.send(
            { receiver_id: selected.other_user_id, content: captionText, target_type: selected.target_type, target_id: selected.target_id },
            token!,
          );
        } catch {
          setMessages((prev) => prev.filter((m) => m.id !== optimId));
          toast.error("Failed to send message");
        }
      }
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 py-6">
      <h1 className="text-2xl font-bold text-foreground mb-5">Messages</h1>

      <div className="flex border border-border rounded-lg overflow-hidden bg-white min-h-[500px]">
        {/* Conversation list */}
        <div className="w-72 shrink-0 border-r border-border flex flex-col">
          <div className="p-3 border-b border-border">
            <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              Conversations
            </p>
          </div>
          <div className="flex-1 overflow-y-auto">
            {loading ? (
              <div className="p-3 space-y-2">
                {Array.from({ length: 4 }).map((_, i) => (
                  <Skeleton key={i} className="h-14 rounded-lg" />
                ))}
              </div>
            ) : conversations.length === 0 ? (
              <div className="p-6 text-center">
                <MessageCircle className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
                <p className="text-xs text-muted-foreground">No conversations yet</p>
              </div>
            ) : (
              conversations.map((conv) => {
                const unread = conv.unread_count > 0;
                const isSelectedConv =
                  selected?.other_user_id === conv.other_user_id &&
                  selected?.target_id === conv.target_id;
                return (
                  <button
                    key={`${conv.other_user_id}-${conv.target_id}`}
                    type="button"
                    onClick={() => selectConversation(conv)}
                    className={cn(
                      "w-full text-left px-3 py-3 flex items-start gap-3 hover:bg-muted/50 transition-colors border-b border-border/50",
                      isSelectedConv && "bg-primary/5 border-l-2 border-l-primary",
                      unread && !isSelectedConv && "bg-blue-50/50",
                    )}
                  >
                    <Avatar className="h-9 w-9 shrink-0">
                      <AvatarFallback className="bg-primary/10 text-primary text-xs font-semibold">
                        {conv.other_username.slice(0, 2).toUpperCase()}
                      </AvatarFallback>
                    </Avatar>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-1">
                        <span className={cn("text-sm truncate", unread ? "font-semibold text-foreground" : "font-medium text-foreground")}>
                          {conv.other_username}
                        </span>
                        <span className="text-xs text-muted-foreground shrink-0">
                          {formatConvTime(conv.last_message_at)}
                        </span>
                      </div>
                      <div className="flex items-center justify-between gap-1 mt-0.5">
                        <p className={cn("text-xs truncate", unread ? "text-foreground font-medium" : "text-muted-foreground")}>
                          {isImageUrl(conv.last_message) ? "📷 Image" : conv.last_message}
                        </p>
                        {unread && (
                          <span className="text-xs bg-primary text-white rounded-full h-4 w-4 flex items-center justify-center shrink-0 font-medium">
                            {conv.unread_count > 9 ? "9+" : conv.unread_count}
                          </span>
                        )}
                      </div>
                      <p className="text-xs text-muted-foreground/60 mt-0.5 capitalize">
                        via {conv.target_type}
                      </p>
                    </div>
                  </button>
                );
              })
            )}
          </div>
        </div>

        {/* Message thread */}
        <div className="flex-1 flex flex-col">
          {!selected ? (
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center">
                <MessageCircle className="h-10 w-10 text-muted-foreground mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">
                  Select a conversation to start messaging
                </p>
              </div>
            </div>
          ) : (
            <>
              {/* Header */}
              <div className="px-4 py-3 border-b border-border">
                <p className="font-semibold text-sm text-foreground">{selected.other_username}</p>
                <p className="text-xs text-muted-foreground capitalize">via {selected.target_type}</p>
              </div>

              {/* Messages */}
              <div className="flex-1 overflow-y-auto p-4 space-y-3">
                {messages.map((msg, i) => {
                  const isMe = msg.sender_id === user?.id;
                  const msgDate = new Date(msg.created_at);
                  const prevDate = i > 0 ? new Date(messages[i - 1].created_at) : null;
                  const showDateSep =
                    !prevDate ||
                    format(msgDate, "yyyy-MM-dd") !== format(prevDate, "yyyy-MM-dd");
                  const isImg = isImageUrl(msg.content);
                  const isLocalPreview = msg.content.startsWith("blob:");
                  return (
                    <div key={msg.id}>
                      {showDateSep && (
                        <div className="flex items-center gap-2 my-2">
                          <div className="flex-1 h-px bg-border" />
                          <span className="text-xs text-muted-foreground shrink-0">
                            {isToday(msgDate) ? "Today" : format(msgDate, "d MMM yyyy")}
                          </span>
                          <div className="flex-1 h-px bg-border" />
                        </div>
                      )}
                      <div className={cn("flex", isMe ? "justify-end" : "justify-start")}>
                        <div className={cn("max-w-xs", !isImg && !isLocalPreview && "px-3 py-2 rounded-2xl text-sm",
                          !isImg && !isLocalPreview && (isMe ? "bg-primary text-white rounded-br-sm" : "bg-muted text-foreground rounded-bl-sm"))}>
                          {isImg ? (
                            <a href={msg.content.startsWith("http") ? msg.content : `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:7878"}${msg.content}`} target="_blank" rel="noreferrer">
                              <img
                                src={msg.content.startsWith("http") ? msg.content : `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:7878"}${msg.content}`}
                                alt="shared image"
                                className="rounded-xl max-w-60 max-h-75 object-cover border border-border"
                              />
                            </a>
                          ) : isLocalPreview ? (
                            <div className="relative">
                              <img
                                src={msg.content}
                                alt="uploading…"
                                className="rounded-xl max-w-60 max-h-75 object-cover border border-border opacity-60"
                              />
                              <div className="absolute inset-0 flex items-center justify-center rounded-xl bg-black/20">
                                <span className="text-white text-xs font-medium">Uploading…</span>
                              </div>
                            </div>
                          ) : (
                            <p className="whitespace-pre-wrap wrap-break-word">{msg.content}</p>
                          )}
                          <p className={cn("text-xs mt-1",
                            isImg || isLocalPreview ? "text-muted-foreground text-right" : (isMe ? "text-white/60" : "text-muted-foreground"))}>
                            {format(msgDate, "h:mm a")}
                          </p>
                        </div>
                      </div>
                    </div>
                  );
                })}
                <div ref={bottomRef} />
              </div>

              {/* Image preview strip */}
              {pendingImage && (
                <div className="px-4 py-2 border-t border-border bg-muted/20">
                  <div className="relative inline-block">
                    <img
                      src={pendingImage.preview}
                      alt="attachment preview"
                      className="h-20 w-20 object-cover rounded-lg border border-border"
                    />
                    <button
                      onClick={removePendingImage}
                      className="absolute -top-1.5 -right-1.5 bg-foreground text-background rounded-full h-4 w-4 flex items-center justify-center hover:bg-red-500 hover:text-white transition-colors"
                    >
                      <X className="h-2.5 w-2.5" />
                    </button>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">Image ready to send — add a caption or press send</p>
                </div>
              )}

              {/* Input */}
              <form
                onSubmit={sendMessage}
                className="px-4 py-3 border-t border-border flex gap-2 items-center"
              >
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  className="hidden"
                  onChange={onImagePick}
                />
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="shrink-0 text-muted-foreground hover:text-foreground"
                  onClick={() => fileInputRef.current?.click()}
                  disabled={sending}
                  title="Attach image"
                >
                  <ImagePlus className="h-5 w-5" />
                </Button>
                <Input
                  value={msgInput}
                  onChange={(e) => setMsgInput(e.target.value)}
                  placeholder={pendingImage ? "Add a caption (optional)…" : "Type a message…"}
                  className="flex-1"
                  disabled={sending}
                />
                <Button
                  type="submit"
                  size="icon"
                  disabled={sending || (!msgInput.trim() && !pendingImage)}
                >
                  <Send className="h-4 w-4" />
                </Button>
              </form>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

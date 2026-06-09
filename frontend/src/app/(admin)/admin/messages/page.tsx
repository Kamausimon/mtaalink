"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Conversation, type Message, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { toast } from "sonner";
import { MessageCircle, Send, ChevronLeft } from "lucide-react";
import { format, parseISO, isToday, isYesterday } from "date-fns";
import { cn } from "@/lib/utils";

const BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:7878";

function isImageUrl(content: string) {
  return content.startsWith("/uploads/") &&
    /\.(jpg|jpeg|png|webp|heic|gif)$/i.test(content);
}

function fmtTime(iso: string) {
  try {
    const d = parseISO(iso);
    if (isToday(d)) return format(d, "HH:mm");
    if (isYesterday(d)) return `Yesterday ${format(d, "HH:mm")}`;
    return format(d, "d MMM HH:mm");
  } catch { return iso; }
}

export default function AdminMessagesPage() {
  const { token, user, _hasHydrated } = useAuthStore();
  const router = useRouter();

  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [loading, setLoading] = useState(true);
  const [active, setActive] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [threadLoading, setThreadLoading] = useState(false);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!_hasHydrated) return;
    if (!token) { router.replace("/login"); return; }

    api.messages.conversations(token)
      .then((r) => setConversations(r.conversations))
      .catch((e) => {
        if (e instanceof ApiError && e.status === 401) router.replace("/login");
        else toast.error("Failed to load conversations");
      })
      .finally(() => setLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [_hasHydrated]);

  async function openConversation(conv: Conversation) {
    setActive(conv);
    setMessages([]);
    setDraft("");
    setThreadLoading(true);
    try {
      const r = await api.messages.get(token!, {
        other_user_id: conv.other_user_id,
        target_type: conv.target_type,
        target_id: conv.target_id,
      });
      setMessages(r.messages);
      // mark unread as read
      const unreadIds = r.messages.filter(m => !m.is_read && m.receiver_id === user?.id).map(m => m.id);
      if (unreadIds.length > 0) {
        api.messages.markRead(unreadIds, token!).catch(() => {});
        setConversations(prev =>
          prev.map(c =>
            c.other_user_id === conv.other_user_id && c.target_id === conv.target_id
              ? { ...c, unread_count: 0 }
              : c,
          ),
        );
      }
    } catch {
      toast.error("Failed to load messages");
    } finally {
      setThreadLoading(false);
    }
  }

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function sendReply() {
    if (!active || !draft.trim() || !token) return;
    setSending(true);
    try {
      await api.messages.send(
        {
          receiver_id: active.other_user_id,
          content: draft.trim(),
          target_type: active.target_type,
          target_id: active.target_id,
        },
        token,
      );
      // Optimistically append
      const optimistic: Message = {
        id: Date.now(),
        sender_id: user!.id,
        receiver_id: active.other_user_id,
        content: draft.trim(),
        is_read: false,
        created_at: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, optimistic]);
      setDraft("");
      // Update conversation last message
      setConversations((prev) =>
        prev.map((c) =>
          c.other_user_id === active.other_user_id && c.target_id === active.target_id
            ? { ...c, last_message: optimistic.content, last_message_at: optimistic.created_at }
            : c,
        ),
      );
    } catch {
      toast.error("Failed to send message");
    } finally {
      setSending(false);
    }
  }

  if (!_hasHydrated || loading) {
    return (
      <div className="space-y-3 max-w-4xl">
        <Skeleton className="h-8 w-40" />
        {Array.from({ length: 5 }).map((_, i) => <Skeleton key={i} className="h-14 rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="flex gap-4 h-[calc(100vh-9rem)] max-w-5xl">
      {/* Conversation list */}
      <div className={cn(
        "flex flex-col border border-border rounded-xl overflow-hidden bg-white",
        active ? "hidden md:flex md:w-72 shrink-0" : "flex-1 md:flex-none md:w-72",
      )}>
        <div className="px-4 py-3 border-b border-border">
          <h1 className="text-base font-semibold">Admin Messages</h1>
          <p className="text-xs text-muted-foreground">{conversations.length} conversation{conversations.length !== 1 ? "s" : ""}</p>
        </div>
        {conversations.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center gap-2 text-center px-4">
            <MessageCircle className="h-8 w-8 text-muted-foreground/40" />
            <p className="text-sm text-muted-foreground">No messages yet.</p>
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto divide-y divide-border">
            {conversations.map((conv) => (
              <button
                key={`${conv.other_user_id}-${conv.target_id}`}
                className={cn(
                  "w-full text-left px-4 py-3 hover:bg-muted/40 transition-colors",
                  active?.other_user_id === conv.other_user_id && active?.target_id === conv.target_id
                    ? "bg-primary/5 border-l-2 border-primary"
                    : "",
                )}
                onClick={() => openConversation(conv)}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium text-sm truncate">{conv.other_username}</span>
                  <span className="text-xs text-muted-foreground shrink-0">{fmtTime(conv.last_message_at)}</span>
                </div>
                <div className="flex items-center gap-1.5 mt-0.5">
                  <p className="text-xs text-muted-foreground truncate flex-1">{isImageUrl(conv.last_message) ? "📷 Image" : conv.last_message}</p>
                  {conv.unread_count > 0 && (
                    <span className="text-xs font-bold bg-primary text-white rounded-full px-1.5 py-0.5 leading-none shrink-0">
                      {conv.unread_count}
                    </span>
                  )}
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Thread panel */}
      {active ? (
        <div className="flex-1 flex flex-col border border-border rounded-xl overflow-hidden bg-white min-w-0">
          {/* Thread header */}
          <div className="px-4 py-3 border-b border-border flex items-center gap-3">
            <button className="md:hidden text-muted-foreground" onClick={() => setActive(null)}>
              <ChevronLeft className="h-5 w-5" />
            </button>
            <div>
              <p className="font-semibold text-sm">{active.other_username}</p>
              <p className="text-xs text-muted-foreground capitalize">{active.target_type} #{active.target_id}</p>
            </div>
          </div>

          {/* Messages */}
          <div className="flex-1 overflow-y-auto px-4 py-3 space-y-2">
            {threadLoading ? (
              <div className="space-y-2">
                {Array.from({ length: 4 }).map((_, i) => (
                  <Skeleton key={i} className={cn("h-10 rounded-2xl", i % 2 === 0 ? "w-3/5" : "w-2/5 ml-auto")} />
                ))}
              </div>
            ) : messages.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-8">No messages in this conversation yet.</p>
            ) : (
              messages.map((msg) => {
                const isMe = msg.sender_id === user?.id;
                const isImg = isImageUrl(msg.content);
                return (
                  <div key={msg.id} className={cn("flex", isMe ? "justify-end" : "justify-start")}>
                    <div className={cn(
                      "max-w-[70%] text-sm",
                      isImg
                        ? ""
                        : cn("px-3 py-2 rounded-2xl", isMe
                            ? "bg-primary text-primary-foreground rounded-br-sm"
                            : "bg-muted text-foreground rounded-bl-sm"),
                    )}>
                      {isImg ? (
                        <a href={`${BASE_URL}${msg.content}`} target="_blank" rel="noreferrer">
                          <img
                            src={`${BASE_URL}${msg.content}`}
                            alt="shared image"
                            className="rounded-xl max-w-60 max-h-72 object-cover border border-border hover:opacity-90 transition-opacity"
                          />
                        </a>
                      ) : (
                        <p className="whitespace-pre-wrap wrap-break-word">{msg.content}</p>
                      )}
                      <p className={cn("text-[10px] mt-0.5", isImg ? "text-muted-foreground" : (isMe ? "text-primary-foreground/70 text-right" : "text-muted-foreground"))}>
                        {fmtTime(msg.created_at)}
                      </p>
                    </div>
                  </div>
                );
              })
            )}
            <div ref={bottomRef} />
          </div>

          {/* Reply input */}
          <div className="px-4 py-3 border-t border-border flex gap-2 items-end">
            <Textarea
              rows={2}
              placeholder={`Reply to ${active.other_username}…`}
              className="resize-none flex-1"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendReply(); }
              }}
            />
            <Button size="icon" onClick={sendReply} disabled={!draft.trim() || sending}>
              <Send className="h-4 w-4" />
            </Button>
          </div>
        </div>
      ) : (
        <div className="hidden md:flex flex-1 items-center justify-center border border-dashed border-border rounded-xl text-muted-foreground">
          <div className="text-center">
            <MessageCircle className="h-10 w-10 mx-auto mb-2 opacity-30" />
            <p className="text-sm">Select a conversation to read and reply</p>
          </div>
        </div>
      )}
    </div>
  );
}

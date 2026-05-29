"use client";

import { useEffect, useState, useRef } from "react";
import { useRouter } from "next/navigation";
import { useAuthStore } from "@/store/auth";
import { api, type Conversation, type Message } from "@/lib/api";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Send, MessageCircle } from "lucide-react";
import { format } from "date-fns";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

export default function MessagesPage() {
  const { token, user, isAuthenticated } = useAuthStore();
  const router = useRouter();
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [selected, setSelected] = useState<Conversation | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [msgInput, setMsgInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isAuthenticated) {
      router.push("/login");
      return;
    }
    api.messages
      .conversations(token!)
      .then((r) => setConversations(r.conversations))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [isAuthenticated, token, router]);

  useEffect(() => {
    if (!selected) return;
    api.messages
      .get(token!, {
        other_user_id: selected.other_user_id,
        target_type: selected.target_type,
        target_id: selected.target_id,
      })
      .then((r) => setMessages(r.messages))
      .catch(() => {});
  }, [selected, token]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function sendMessage(e: React.FormEvent) {
    e.preventDefault();
    if (!msgInput.trim() || !selected) return;
    setSending(true);
    try {
      await api.messages.send(
        {
          receiver_id: selected.other_user_id,
          content: msgInput.trim(),
          target_type: selected.target_type,
          target_id: selected.target_id,
        },
        token!,
      );
      setMsgInput("");
      // Reload messages
      const r = await api.messages.get(token!, {
        other_user_id: selected.other_user_id,
        target_type: selected.target_type,
        target_id: selected.target_id,
      });
      setMessages(r.messages);
    } catch {
      toast.error("Failed to send message");
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
              conversations.map((conv) => (
                <button
                  key={`${conv.other_user_id}-${conv.target_id}`}
                  type="button"
                  onClick={() => setSelected(conv)}
                  className={cn(
                    "w-full text-left px-3 py-3 flex items-start gap-3 hover:bg-muted/50 transition-colors border-b border-border/50",
                    selected?.other_user_id === conv.other_user_id &&
                      "bg-primary/5 border-l-2 border-l-primary",
                  )}
                >
                  <Avatar className="h-9 w-9 shrink-0">
                    <AvatarFallback className="bg-primary/10 text-primary text-xs font-semibold">
                      {conv.other_username.slice(0, 2).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium text-foreground truncate">
                        {conv.other_username}
                      </span>
                      {conv.unread_count > 0 && (
                        <span className="text-xs bg-primary text-white rounded-full h-5 w-5 flex items-center justify-center shrink-0 ml-1">
                          {conv.unread_count}
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground truncate mt-0.5">
                      {conv.last_message}
                    </p>
                  </div>
                </button>
              ))
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
                <p className="font-semibold text-sm text-foreground">
                  {selected.other_username}
                </p>
              </div>

              {/* Messages */}
              <div className="flex-1 overflow-y-auto p-4 space-y-3">
                {messages.map((msg) => {
                  const isMe = msg.sender_id === user?.id;
                  return (
                    <div
                      key={msg.id}
                      className={cn("flex", isMe ? "justify-end" : "justify-start")}
                    >
                      <div
                        className={cn(
                          "max-w-xs px-3 py-2 rounded-2xl text-sm",
                          isMe
                            ? "bg-primary text-white rounded-br-sm"
                            : "bg-muted text-foreground rounded-bl-sm",
                        )}
                      >
                        <p>{msg.content}</p>
                        <p
                          className={cn(
                            "text-xs mt-1",
                            isMe ? "text-white/60" : "text-muted-foreground",
                          )}
                        >
                          {format(new Date(msg.created_at), "h:mm a")}
                        </p>
                      </div>
                    </div>
                  );
                })}
                <div ref={bottomRef} />
              </div>

              {/* Input */}
              <form
                onSubmit={sendMessage}
                className="px-4 py-3 border-t border-border flex gap-2"
              >
                <Input
                  value={msgInput}
                  onChange={(e) => setMsgInput(e.target.value)}
                  placeholder="Type a message…"
                  className="flex-1"
                  disabled={sending}
                />
                <Button type="submit" size="icon" disabled={sending || !msgInput.trim()}>
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

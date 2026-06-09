const BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:7878";

type RequestOptions = {
  method?: string;
  body?: unknown;
  token?: string | null;
};

async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const { method = "GET", body, token } = opts;

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  if (token) headers["Authorization"] = `Bearer ${token}`;

  const res = await fetch(`${BASE_URL}${path}`, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({}));
    throw new ApiError(res.status, err.message ?? err.error ?? res.statusText ?? "Something went wrong");
  }

  return res.json() as Promise<T>;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

// ── Auth ─────────────────────────────────────────────────────────────────────

// Raw flat shape the backend returns for auth endpoints
type RawAuthResponse = {
  token: string;
  user_id: number;
  username: string;
  role: string;
};

export const api = {
  auth: {
    register: async (data: {
      username: string;
      email: string;
      password: string;
      confirm_password: string;
      role: string;
      service_description?: string;
      business_name?: string;
    }): Promise<{ token: string; user: User }> => {
      const raw = await request<RawAuthResponse>("/auth/register", { method: "POST", body: data });
      return {
        token: raw.token,
        user: { id: raw.user_id, username: raw.username, email: data.email, role: raw.role as User["role"] },
      };
    },

    login: async (data: { email: string; password: string }): Promise<{ token: string; user: User }> => {
      const raw = await request<RawAuthResponse>("/auth/login", { method: "POST", body: data });
      return {
        token: raw.token,
        user: { id: raw.user_id, username: raw.username, email: data.email, role: raw.role as User["role"] },
      };
    },

    me: (token: string) => request<{ user: User }>("/auth/me", { token }),

    forgotPassword: (email: string) =>
      request("/auth/forgot-password", { method: "POST", body: { email } }),

    resetPassword: (token: string, password: string) =>
      request("/auth/reset-password", { method: "POST", body: { token, password } }),
  },

  // ── Dashboard ───────────────────────────────────────────────────────────
  dashboard: {
    get: (token: string) => request<DashboardData>("/dashboard", { token }),
  },

  // ── Search ──────────────────────────────────────────────────────────────
  search: {
    query: async (params: SearchParams): Promise<SearchResults> => {
      const qs = new URLSearchParams();
      if (params.q) qs.set("q", params.q);
      if (params.category) qs.set("category", params.category);
      if (params.lat) qs.set("lat", String(params.lat));
      if (params.lng) qs.set("lng", String(params.lng));
      if (params.radius_km) qs.set("radius_km", String(params.radius_km));
      if (params.page) qs.set("page", String(params.page));
      if (params.per_page) qs.set("per_page", String(params.per_page));

      // Backend returns separate providers/businesses arrays with different
      // field names (average_rating, service_name, business_name). Transform
      // them into the unified SearchResult shape the UI consumes.
      const raw = await request<RawSearchResponse>(`/search?${qs}`);

      const results: SearchResult[] = [
        ...(raw.providers ?? []).map((p) => ({
          id: p.id,
          type: "provider" as const,
          name: p.service_name ?? "Provider",
          description: p.service_description,
          category: p.category,
          location: p.location,
          profile_photo: p.profile_photo,
          avg_rating: p.average_rating,
          review_count: p.review_count,
          distance_km: p.distance_km,
        })),
        ...(raw.businesses ?? []).map((b) => ({
          id: b.id,
          type: "business" as const,
          name: b.business_name,
          description: b.description,
          category: b.category,
          location: b.location,
          profile_photo: b.profile_photo ?? b.logo,
          avg_rating: b.average_rating,
          review_count: b.review_count,
          distance_km: b.distance_km,
        })),
      ];

      return { results, total: raw.total };
    },
  },

  // ── Providers ───────────────────────────────────────────────────────────
  providers: {
    list: (params?: { category?: string; location?: string }) => {
      const qs = new URLSearchParams();
      if (params?.category) qs.set("category", params.category);
      if (params?.location) qs.set("location", params.location);
      return request<{ providers: PublicProvider[] }>(`/service_providers/listProviders?${qs}`);
    },
    getById: (id: number) =>
      request<{ provider: ProviderProfile; services: Service[] }>(`/service_providers/${id}`),
    onboard: (data: ProviderOnboardInput, token: string) =>
      request("/service_providers/onboard", { method: "POST", body: data, token }),
    updateProfile: (data: Partial<ProviderOnboardInput>, token: string) =>
      request("/service_providers/updateProfile", { method: "POST", body: data, token }),
    getMyData: (token: string) =>
      request<{ provider_data: ProviderProfile }>("/service_providers/getProviderData", { token }),
    availability: {
      get: (id: number) =>
        request<{ schedule: Availability[] }>(`/availability/provider/${id}`),
      set: (
        id: number,
        schedule: Array<{ day: string; is_available: boolean; start_time?: string; end_time?: string }>,
        token: string,
      ) => request(`/availability/provider/${id}`, { method: "PUT", body: schedule, token }),
      slots: (id: number, date: string, slotMinutes = 60) =>
        request<{ slots: string[] }>(`/availability/provider/${id}/slots?date=${date}&slot_minutes=${slotMinutes}`),
    },
  },

  // ── Businesses ──────────────────────────────────────────────────────────
  businesses: {
    list: (params?: { category?: string; location?: string }) => {
      const qs = new URLSearchParams();
      if (params?.category) qs.set("category", params.category);
      if (params?.location) qs.set("location", params.location);
      return request<{ businesses: PublicBusiness[] }>(`/businesses/listBusinesses?${qs}`);
    },
    getById: (id: number) =>
      request<{ business: BusinessProfile; services: Service[]; branches: Branch[] }>(`/businesses/${id}`),
    onboard: (data: BusinessOnboardInput, token: string) =>
      request("/businesses/onboard", { method: "POST", body: data, token }),
  },

  // ── Bookings ────────────────────────────────────────────────────────────
  bookings: {
    create: (data: CreateBookingInput, token: string) =>
      request<{ booking_id: number }>("/bookings/createBooking", { method: "POST", body: data, token }),
    myBookings: (token: string, params?: { status?: string; target_type?: string }) => {
      const qs = new URLSearchParams();
      if (params?.status) qs.set("status", params.status);
      if (params?.target_type) qs.set("target_type", params.target_type);
      return request<{ bookings: Booking[] }>(`/bookings/getBookings/me?${qs}`, { token });
    },
    received: (token: string, params: { target_type: string; target_id: number; status?: string }) => {
      const qs = new URLSearchParams({
        target_type: params.target_type,
        target_id: String(params.target_id),
      });
      if (params.status && params.status !== "all") qs.set("status", params.status);
      return request<{ bookings: BookingReceived[] }>(`/bookings/getBookings/received?${qs}`, { token });
    },
    getById: (id: number, token: string) =>
      request<{ booking: Booking }>(`/bookings/${id}`, { token }),
    updateStatus: (id: number, status: string, token: string, cancelReason?: string, disputeReason?: string) =>
      request(`/bookings/${id}/status`, {
        method: "POST",
        body: { status, cancel_reason: cancelReason, dispute_reason: disputeReason },
        token,
      }),
    reschedule: (id: number, scheduledTime: string, token: string) =>
      request(`/bookings/${id}/reschedule`, {
        method: "POST",
        body: { scheduled_time: scheduledTime },
        token,
      }),
    delete: (id: number, token: string) =>
      request(`/bookings/${id}/delete`, { method: "POST", token }),
    submitDisputeResponse: (id: number, response: string, token: string) =>
      request(`/bookings/${id}/dispute_response`, { method: "POST", body: { response }, token }),
    uploadEvidence: async (id: number, file: File, caption: string, token: string): Promise<{ url: string }> => {
      const form = new FormData();
      form.append("file", file);
      if (caption.trim()) form.append("caption", caption.trim());
      const res = await fetch(`${BASE_URL}/bookings/${id}/evidence`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        body: form,
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new ApiError(res.status, err.message ?? "Upload failed");
      }
      return res.json();
    },
    getEvidence: (id: number, token: string) =>
      request<{ evidence: DisputeEvidence[] }>(`/bookings/${id}/evidence`, { token }),
  },

  // ── Payments ────────────────────────────────────────────────────────────
  payments: {
    initiate: (data: { booking_id: number; phone_number: string; amount: number }, token: string) =>
      request("/payments/initiate", { method: "POST", body: data, token }),
    status: (bookingId: number, token: string) =>
      request(`/payments/booking/${bookingId}`, { token }),
  },

  // ── Reviews ─────────────────────────────────────────────────────────────
  reviews: {
    create: (
      data: { comment: string; rating: number },
      target_type: string,
      target_id: number,
      token: string,
    ) =>
      request(`/reviews/createReviews?target_type=${target_type}&target_id=${target_id}`, {
        method: "POST",
        body: data,
        token,
      }),
    get: (target_type: string, target_id: number) =>
      request<{ reviews: Review[] }>(`/reviews/getReviews?target_type=${target_type}&target_id=${target_id}`),
    aggregate: (target_type: string, target_id: number) =>
      request<{ aggregated_rating: { average_rating: number; review_count: number } }>(
        `/reviews/getReviewAggById?target_type=${target_type}&target_id=${target_id}`,
      ),
    flag: (reviewId: number, reason: string, token: string) =>
      request(`/reviews/${reviewId}/flag`, { method: "POST", body: { reason }, token }),
  },

  // ── Messages ────────────────────────────────────────────────────────────
  messages: {
    conversations: (token: string) =>
      request<{ conversations: Conversation[] }>("/messages/conversations", { token }),
    get: (token: string, params: { other_user_id: number; target_type: string; target_id: number }) => {
      const qs = new URLSearchParams({
        other_user_id: String(params.other_user_id),
        target_type: params.target_type,
        target_id: String(params.target_id),
      });
      return request<{ messages: Message[] }>(`/messages/getMessages?${qs}`, { token });
    },
    send: (
      data: { receiver_id: number; content: string; target_type: string; target_id: number },
      token: string,
    ) => request("/messages/sendMessage", { method: "POST", body: data, token }),
    unreadCount: (token: string) =>
      request<{ unread_count: number }>("/messages/unreadMessagesCount", { token }),
    markRead: (messageIds: number[], token: string) =>
      request("/messages/markMessagesAsRead", { method: "POST", body: { message_ids: messageIds }, token }),
    uploadAttachment: async (file: File, token: string): Promise<{ url: string }> => {
      const form = new FormData();
      form.append("file", file);
      const res = await fetch(`${BASE_URL}/messages/upload`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        body: form,
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new ApiError(res.status, err.message ?? "Upload failed");
      }
      return res.json();
    },
  },

  // ── Notifications ────────────────────────────────────────────────────────
  notifications: {
    list: (token: string, params?: { page?: number; unread_only?: boolean }) => {
      const qs = new URLSearchParams();
      if (params?.page) qs.set("page", String(params.page));
      if (params?.unread_only) qs.set("unread_only", "true");
      return request<{ notifications: Notification[]; total: number }>(`/notifications?${qs}`, { token });
    },
    unreadCount: (token: string) =>
      request<{ unread_count: number }>("/notifications/unread-count", { token }),
    markAllRead: (token: string) =>
      request("/notifications/read-all", { method: "POST", token }),
    markRead: (id: number, token: string) =>
      request(`/notifications/${id}/read`, { method: "POST", token }),
  },

  // ── Services ────────────────────────────────────────────────────────────
  services: {
    list: (targetType: string, targetId: number, token: string) =>
      request<{ services: ManagedService[] }>(
        `/services/getServices?target_type=${targetType}&target_id=${targetId}`, { token }),
    create: (data: CreateServiceInput, token: string) =>
      request<{ service_id: number }>("/services/createService", { method: "POST", body: data, token }),
    update: (data: UpdateServiceInput, token: string) =>
      request("/services/updateService", { method: "POST", body: data, token }),
    delete: (serviceId: number, token: string) =>
      request("/services/deleteService", { method: "POST", body: { service_id: serviceId }, token }),
  },

  // ── Analytics ───────────────────────────────────────────────────────────
  analytics: {
    get: (targetType: string, targetId: number, days: number, token: string) =>
      request<AnalyticsData>(`/analytics/${targetType}/${targetId}?days=${days}`, { token }),
  },

  // ── Wallet ──────────────────────────────────────────────────────────────
  wallet: {
    get: (target_type: string, target_id: number, token: string) =>
      request<{ wallet: Wallet }>(`/wallet/${target_type}/${target_id}`, { token }),
    transactions: (target_type: string, target_id: number, token: string) =>
      request<{ transactions: Transaction[] }>(`/wallet/${target_type}/${target_id}/transactions`, { token }),
    requestPayout: (
      target_type: string,
      target_id: number,
      data: { amount: number; phone_number: string },
      token: string,
    ) =>
      request(`/wallet/${target_type}/${target_id}/payout`, { method: "POST", body: data, token }),
  },

  // ── Favorites ────────────────────────────────────────────────────────────
  favorites: {
    add: (data: { target_type: string; target_id: number }, token: string) =>
      request("/favorites/addFavorite", { method: "POST", body: data, token }),
    list: (token: string) =>
      request<{ favorites: { target_type: string; target_id: number }[] }>("/favorites/getFavorites", { token }),
    remove: (target_id: number, target_type: string, token: string) =>
      request(`/favorites/removeFavorite/${target_id}?target_type=${target_type}`, { method: "POST", token }),
  },

  // ── Categories ───────────────────────────────────────────────────────────
  categories: {
    all: () => request<{ categories: Category[] }>("/categories/allCategories"),
  },

  // ── Posts ─────────────────────────────────────────────────────────────────
  posts: {
    byProvider: (providerId: number) =>
      request<{ posts: Post[] }>(`/posts/provider/${providerId}/posts`),
    byBusiness: (businessId: number) =>
      request<{ posts: Post[] }>(`/posts/business/${businessId}/posts`),
    create: (data: { title: string; content: string; provider_id?: number; business_id?: number }, token: string) =>
      request<{ post_id: number }>("/posts/createPosts", { method: "POST", body: data, token }),
    update: (id: number, data: { title?: string; content?: string; attachments: string[] }, token: string) =>
      request(`/posts/updatePost/${id}`, { method: "POST", body: data, token }),
    delete: (id: number, token: string) =>
      request(`/posts/deletePost/${id}`, { method: "POST", token }),
    like: (id: number, token: string) =>
      request(`/posts/${id}/like`, { method: "POST", token }),
    unlike: (id: number, token: string) =>
      request(`/posts/${id}/like`, { method: "DELETE", token }),
    comments: (id: number) =>
      request<{ comments: PostComment[]; likes: number }>(`/posts/${id}/comments`),
    addComment: (id: number, comment: string, token: string) =>
      request(`/posts/${id}/comments`, { method: "POST", body: { comment }, token }),
    deleteComment: (postId: number, commentId: number, token: string) =>
      request(`/posts/${postId}/comments/${commentId}`, { method: "DELETE", token }),
  },

  admin: {
    dashboard: (token: string) =>
      request<AdminDashboardStats>("/admin/dashboard", { token }),
    users: (token: string) =>
      request<{ users: AdminUser[] }>("/admin/users", { token }),
    deleteUser: (userId: number, token: string) =>
      request("/admin/delete_user", { method: "POST", body: { user_id: userId }, token }),
    userAnalytics: (token: string) =>
      request<AdminUserAnalytics>("/admin/userAnalytics", { token }),
    payouts: (token: string) =>
      request<{ pending_payouts: AdminPayout[] }>("/admin/payouts", { token }),
    approvePayout: (id: number, notes: string | null, token: string) =>
      request(`/admin/payouts/${id}/approve`, { method: "POST", body: { notes }, token }),
    rejectPayout: (id: number, notes: string | null, token: string) =>
      request(`/admin/payouts/${id}/reject`, { method: "POST", body: { notes }, token }),
    categories: (token: string) =>
      request<{ categories: Category[] }>("/admin/categories", { token }),
    createCategory: (name: string, parentId: number | null, token: string) =>
      request("/admin/create_category", { method: "POST", body: { name, parent_id: parentId }, token }),
    deleteCategory: (categoryId: number, token: string) =>
      request("/admin/delete_category", { method: "POST", body: { category_id: categoryId }, token }),
    flaggedReviews: (token: string) =>
      request<{ flagged_reviews: FlaggedReview[] }>("/admin/moderateReviews", { token }),
    resolveFlag: (reviewId: number, token: string) =>
      request("/admin/resolveFlag", { method: "POST", body: { review_id: reviewId }, token }),
    disputes: (token: string) =>
      request<{ disputes: AdminDispute[] }>("/admin/disputes", { token }),
    resolveDispute: (bookingId: number, resolution: "completed" | "cancelled", note: string | null, token: string) =>
      request(`/admin/disputes/${bookingId}/resolve`, { method: "POST", body: { resolution, note }, token }),
    suspend: (entityType: "provider" | "business", entityId: number, days: number, token: string) =>
      request(`/admin/suspend/${entityType}/${entityId}`, { method: "POST", body: { days }, token }),
    unsuspend: (entityType: "provider" | "business", entityId: number, token: string) =>
      request(`/admin/unsuspend/${entityType}/${entityId}`, { method: "POST", token }),
  },
};

// ── Types ─────────────────────────────────────────────────────────────────────

export type User = {
  id: number;
  username: string;
  email: string;
  role: "client" | "provider" | "business" | "admin";
};

export type DashboardData = {
  user_id: number;
  username: string;
  email: string;
  role: string;
  unread_notifications: number;
  upcoming_bookings: number;
  provider_id?: number;
  business_id?: number;
  pending_bookings?: number;
  balance?: string;
  total_earned?: string;
};

export type SearchParams = {
  q?: string;
  category?: string;
  lat?: number;
  lng?: number;
  radius_km?: number;
  page?: number;
  per_page?: number;
};

export type SearchResult = {
  id: number;
  type: "provider" | "business";
  name: string;
  description?: string;
  category?: string;
  location?: string;
  profile_photo?: string;
  avg_rating?: number;
  review_count?: number;
  distance_km?: number;
};

export type SearchResults = { results: SearchResult[]; total: number };

// Raw shape returned by the backend /search endpoint before transformation
type RawProviderResult = {
  id: number;
  service_name?: string;
  service_description?: string;
  category?: string;
  location?: string;
  profile_photo?: string;
  average_rating: number;
  review_count: number;
  distance_km?: number;
};

type RawBusinessResult = {
  id: number;
  business_name: string;
  description?: string;
  category?: string;
  location?: string;
  profile_photo?: string;
  logo?: string;
  average_rating: number;
  review_count: number;
  distance_km?: number;
};

type RawSearchResponse = {
  providers: RawProviderResult[];
  businesses: RawBusinessResult[];
  total: number;
  page: number;
  per_page: number;
};

export type PublicProvider = {
  id: number;
  user_id?: number;
  service_name: string;
  category?: string;
  location?: string;
  email?: string;
  phone_number?: string;
  website?: string;
  profile_photo?: string;
  avg_rating?: number;
  review_count?: number;
};

export type ProviderProfile = PublicProvider & {
  service_description?: string;
  whatsapp?: string;
  cover_photo?: string;
};

export type PublicBusiness = {
  id: number;
  user_id?: number;
  business_name: string;
  description?: string;
  category?: string;
  location?: string;
  phone_number?: string;
  email?: string;
  website?: string;
  profile_photo?: string;
  logo?: string;
  avg_rating?: number;
  review_count?: number;
};

export type BusinessProfile = PublicBusiness & {
  whatsapp?: string;
  cover_photo?: string;
};

export type Service = {
  id: number;
  title: string;
  description?: string;
  price?: number;
  duration?: number;
  category_id?: number;
};

export type ManagedService = {
  id: number;
  target_id: number;
  target_type: string;
  title: string;
  description: string;
  price: string;
  duration: number;
  category_id?: number;
  is_active: boolean;
};

export type CreateServiceInput = {
  target_id: number;
  target_type: string;
  title: string;
  description: string;
  price: number;
  duration: number;
  category_id?: number;
  is_active: boolean;
};

export type UpdateServiceInput = {
  service_id: number;
  target_id: number;
  target_type: string;
  title?: string;
  description?: string;
  price?: number;
  duration?: number;
  category_id?: number;
  is_active?: boolean;
};

export type Branch = {
  id: number;
  name: string;
  address?: string;
  phone?: string;
  latitude?: number;
  longitude?: number;
  ward?: string;
  constituency?: string;
  county?: string;
};

export type Availability = {
  id: number;
  day: string;
  start_time: string;
  end_time: string;
  is_available: boolean;
};

export type Booking = {
  id: number;
  client_id: number;
  target_type: string;
  target_id: number;
  service_id?: number;
  service_description?: string;
  scheduled_time: string;
  status: string;
  duration?: number;
  client_address?: string;
  client_latitude?: number;
  client_longitude?: number;
  client_phone?: string;
  cancel_reason?: string;
  dispute_reason?: string;
  dispute_response?: string;
  admin_resolution?: string;
  created_at?: string;
};

export type BookingReceived = Booking & {
  client_name: string;
  client_email: string;
  service_name?: string;
};

export type CreateBookingInput = {
  target_type: string;
  target_id: number;
  service_id?: number;
  service_description: string;
  scheduled_time: string;
  client_phone?: string;
  client_address?: string;
  client_latitude?: number;
  client_longitude?: number;
};

export type Review = {
  id: number;
  reviewer_id: number;
  rating: number;
  comment: string;
  verified: boolean;
  created_at: string;
};

export type Message = {
  id: number;
  sender_id: number;
  receiver_id: number;
  content: string;
  created_at: string;
  is_read: boolean;
  read_at?: string;
};

export type Conversation = {
  other_user_id: number;
  other_username: string;
  target_type: string;
  target_id: number;
  last_message: string;
  last_message_at: string;
  unread_count: number;
};

export type Notification = {
  id: number;
  notif_type: string;
  title: string;
  body: string;
  target_type?: string;
  target_id?: number;
  is_read: boolean;
  created_at: string;
};

export type AnalyticsData = {
  period_days: number;
  overview: {
    total_bookings: number;
    pending: number;
    confirmed: number;
    completed: number;
    cancelled: number;
    total_revenue: number;
    average_rating: number;
    review_count: number;
  };
  bookings_over_time: { date: string; count: number }[];
  revenue_over_time: { date: string; amount: number }[];
  top_services: { service_name: string | null; booking_count: number; revenue: number }[];
  repeat_clients: { total_clients: number; repeat_clients: number; repeat_rate: number };
};

export type Wallet = {
  balance: string;
  total_earned: string;
  total_paid_out: string;
};

export type Transaction = {
  id: number;
  txn_type: "credit" | "debit";
  amount: string;
  description: string;
  created_at: string;
};

export type Category = {
  id: number;
  category_name: string;
  parent_id?: number;
  parent_name?: string;
};

export type ProviderOnboardInput = {
  service_name: string;
  service_description: string;
  category?: string;
  location?: string;
  phone_number?: string;
  email: string;
  website?: string;
  whatsapp?: string;
};

export type BusinessOnboardInput = {
  business_name: string;
  description: string;
  category?: string;
  location?: string;
  license_number: string;
  krapin: string;
  phone_number: string;
  email: string;
  website?: string;
  whatsapp?: string;
};

export type Post = {
  id: number;
  title: string;
  content: string;
  provider_id?: number;
  business_id?: number;
  image_urls: string[];
  like_count: number;
  comment_count: number;
  created_at: string;
  updated_at: string;
};

export type PostComment = {
  id: number;
  user_id: number;
  username: string;
  comment: string;
  created_at: string;
};

// ── Admin types ───────────────────────────────────────────────────────────────

export type AdminDashboardStats = {
  users: { clients: number; providers: number; businesses: number; total: number };
  bookings: { total: number; pending: number; confirmed: number; completed: number; cancelled: number };
  revenue: { total_collected: number; pending_payments: number };
  payouts: { pending_amount: number; approved_amount: number };
};

export type AdminUser = {
  id: number;
  username: string;
  email: string;
  role: string | null;
};

export type AdminPayout = {
  id: number;
  wallet_id: number;
  amount: string;
  phone_number: string;
  status: string;
  notes: string | null;
  target_type: string | null;
  target_id: number | null;
};

export type FlaggedReview = {
  review_id: number;
  reviewer_id: number;
  target_type: string;
  target_id: number;
  rating: number;
  comment: string | null;
  flag_count: number;
};

export type AdminUserAnalytics = {
  users: { clients: number; providers: number; businesses: number; total: number };
  bookings: { pending: number; confirmed: number; completed: number; cancelled: number };
  signups_last_7_days: { day: string; count: number }[];
};

export type DisputeEvidence = {
  id: number;
  uploader_role: "client" | "provider";
  file_url: string;
  caption: string | null;
  created_at: string | null;
};

export type AdminDispute = {
  booking_id: number;
  client_id: number;
  client_username: string;
  service_owner_user_id: number | null;
  target_type: string;
  target_id: number;
  provider_name: string | null;
  service_description: string | null;
  scheduled_time: string;
  dispute_reason: string | null;
  dispute_response: string | null;
  admin_resolution: string | null;
  created_at: string | null;
};

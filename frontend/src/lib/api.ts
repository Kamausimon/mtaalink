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
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new ApiError(res.status, err.error ?? "Something went wrong");
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

export const api = {
  auth: {
    register: (data: {
      username: string;
      email: string;
      password: string;
      confirm_password: string;
      role: string;
      service_description?: string;
      business_name?: string;
    }) => request<{ token: string; user: User }>("/auth/register", { method: "POST", body: data }),

    login: (data: { email: string; password: string }) =>
      request<{ token: string; user: User }>("/auth/login", { method: "POST", body: data }),

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
    query: (params: SearchParams) => {
      const qs = new URLSearchParams();
      if (params.q) qs.set("q", params.q);
      if (params.category) qs.set("category", params.category);
      if (params.lat) qs.set("lat", String(params.lat));
      if (params.lng) qs.set("lng", String(params.lng));
      if (params.radius_km) qs.set("radius_km", String(params.radius_km));
      if (params.page) qs.set("page", String(params.page));
      if (params.per_page) qs.set("per_page", String(params.per_page));
      return request<SearchResults>(`/search?${qs}`);
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
      request<{ provider_data: ProviderProfile }>("/service_providers/getProviderData?provider_id=0", { token }),
    availability: {
      get: (id: number) => request<{ availability: Availability[] }>(`/availability/provider/${id}`),
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
    received: (token: string, params: { target_type: string; target_id: number; status: string }) => {
      const qs = new URLSearchParams({
        target_type: params.target_type,
        target_id: String(params.target_id),
        status: params.status,
      });
      return request<{ bookings: BookingReceived[] }>(`/bookings/getBookings/received?${qs}`, { token });
    },
    getById: (id: number, token: string) =>
      request<{ booking: Booking }>(`/bookings/${id}`, { token }),
    updateStatus: (id: number, status: string, token: string, cancelReason?: string) =>
      request(`/bookings/${id}/status`, {
        method: "POST",
        body: { status, cancel_reason: cancelReason },
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

export type PublicProvider = {
  id: number;
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

export type Wallet = {
  balance: string;
  total_earned: string;
  total_paid_out: string;
};

export type Transaction = {
  id: number;
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

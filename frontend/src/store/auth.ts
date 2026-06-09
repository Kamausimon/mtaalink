"use client";

import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { User } from "@/lib/api";

type AuthState = {
  token: string | null;
  user: User | null;
  isAuthenticated: boolean;
  isAdmin: boolean;
  _hasHydrated: boolean;
  setAuth: (token: string, user: User) => void;
  clearAuth: () => void;
  updateUser: (user: User) => void;
  setIsAdmin: (isAdmin: boolean) => void;
};

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      user: null,
      isAuthenticated: false,
      isAdmin: false,
      _hasHydrated: false,

      setAuth: (token, user) => set({ token, user, isAuthenticated: true }),
      clearAuth: () => set({ token: null, user: null, isAuthenticated: false, isAdmin: false }),
      updateUser: (user) => set({ user }),
      setIsAdmin: (isAdmin) => set({ isAdmin }),
    }),
    {
      name: "mtaalink-auth",
      partialize: (state) => ({ token: state.token, user: state.user, isAdmin: state.isAdmin }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.isAuthenticated = !!state.token;
          state._hasHydrated = true;
        }
      },
    },
  ),
);

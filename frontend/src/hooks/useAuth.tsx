import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import type { User } from '../types';
import { getMe, getOnboardingStatus, logout as apiLogout } from '../api/auth';

interface AuthCtx {
  user: User | null;
  loading: boolean;
  needsOnboarding: boolean;
  refresh: () => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthCtx>({
  user: null,
  loading: true,
  needsOnboarding: false,
  refresh: async () => {},
  logout: async () => {},
});

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [needsOnboarding, setNeedsOnboarding] = useState(false);

  const refresh = async () => {
    try {
      const u = await getMe();
      setUser(u);
      try {
        const status = await getOnboardingStatus();
        setNeedsOnboarding(status.needs_onboarding);
      } catch {
        setNeedsOnboarding(false);
      }
    } catch {
      setUser(null);
      setNeedsOnboarding(false);
    } finally {
      setLoading(false);
    }
  };

  const logout = async () => {
    await apiLogout();
    setUser(null);
    setNeedsOnboarding(false);
  };

  useEffect(() => {
    refresh();
  }, []);

  return (
    <AuthContext.Provider value={{ user, loading, needsOnboarding, refresh, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}

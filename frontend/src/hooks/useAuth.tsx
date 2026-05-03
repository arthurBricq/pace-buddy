import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import type { User } from '../types';
import { getMe, getRunnerProfileStatus, logout as apiLogout } from '../api/auth';

interface AuthCtx {
  user: User | null;
  loading: boolean;
  needsRunnerProfile: boolean;
  refresh: () => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthCtx>({
  user: null,
  loading: true,
  needsRunnerProfile: false,
  refresh: async () => {},
  logout: async () => {},
});

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [needsRunnerProfile, setNeedsRunnerProfile] = useState(false);

  const refresh = async () => {
    try {
      const u = await getMe();
      setUser(u);
      try {
        const status = await getRunnerProfileStatus();
        setNeedsRunnerProfile(status.needs_runner_profile);
      } catch {
        setNeedsRunnerProfile(false);
      }
    } catch {
      setUser(null);
      setNeedsRunnerProfile(false);
    } finally {
      setLoading(false);
    }
  };

  const logout = async () => {
    await apiLogout();
    setUser(null);
    setNeedsRunnerProfile(false);
  };

  useEffect(() => {
    refresh();
  }, []);

  return (
    <AuthContext.Provider value={{ user, loading, needsRunnerProfile, refresh, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}

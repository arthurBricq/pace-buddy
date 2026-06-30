import { createContext } from 'react';
import type { User } from '../types';

export interface AuthCtx {
  user: User | null;
  loading: boolean;
  needsRunnerProfile: boolean;
  refresh: () => Promise<void>;
  logout: () => Promise<void>;
}

export const AuthContext = createContext<AuthCtx>({
  user: null,
  loading: true,
  needsRunnerProfile: false,
  refresh: async () => {},
  logout: async () => {},
});

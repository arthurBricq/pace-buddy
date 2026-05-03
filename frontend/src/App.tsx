import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider } from './hooks/useAuth';
import AuthGuard from './components/AuthGuard';
import AdminGuard from './components/AdminGuard';
import LoginPage from './pages/LoginPage';
import ActivityListPage from './pages/ActivityListPage';
import ActivityDetailPage from './pages/ActivityDetailPage';
import TrainingsListPage from './pages/TrainingsListPage';
import TrainingDetailPage from './pages/TrainingDetailPage';
import RacesPage from './pages/RacesPage';
import ProfilePage from './pages/ProfilePage';
import AdminDashboardPage from './pages/AdminDashboardPage';
import AdminUsersPage from './pages/AdminUsersPage';
import AdminCoachContextPage from './pages/AdminCoachContextPage';
import HelpPage from './pages/HelpPage';
import OnboardingPage from './pages/OnboardingPage';
import RunningCoachPage from './pages/RunningCoachPage';

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/register" element={<Navigate to="/login" replace />} />
          <Route
            path="/activities"
            element={
              <AuthGuard>
                <ActivityListPage />
              </AuthGuard>
            }
          />
          <Route
            path="/activities/:id"
            element={
              <AuthGuard>
                <ActivityDetailPage />
              </AuthGuard>
            }
          />
          <Route path="/strava" element={<Navigate to="/profile" replace />} />
          <Route
            path="/onboarding"
            element={
              <AuthGuard>
                <OnboardingPage />
              </AuthGuard>
            }
          />
          <Route
            path="/trainings"
            element={
              <AuthGuard>
                <TrainingsListPage />
              </AuthGuard>
            }
          />
          <Route
            path="/trainings/:id"
            element={
              <AuthGuard>
                <TrainingDetailPage />
              </AuthGuard>
            }
          />
          <Route
            path="/coach"
            element={
              <AuthGuard>
                <RunningCoachPage />
              </AuthGuard>
            }
          />
          <Route
            path="/races"
            element={
              <AuthGuard>
                <RacesPage />
              </AuthGuard>
            }
          />
          <Route
            path="/profile"
            element={
              <AuthGuard>
                <ProfilePage />
              </AuthGuard>
            }
          />
          <Route
            path="/admin"
            element={
              <AuthGuard>
                <AdminGuard>
                  <AdminDashboardPage />
                </AdminGuard>
              </AuthGuard>
            }
          />
          <Route
            path="/admin/users"
            element={
              <AuthGuard>
                <AdminGuard>
                  <AdminUsersPage />
                </AdminGuard>
              </AuthGuard>
            }
          />
          <Route
            path="/admin/coach-contexts/:userId"
            element={
              <AuthGuard>
                <AdminGuard>
                  <AdminCoachContextPage />
                </AdminGuard>
              </AuthGuard>
            }
          />
          <Route
            path="/help"
            element={
              <AuthGuard>
                <HelpPage />
              </AuthGuard>
            }
          />
          <Route path="*" element={<Navigate to="/coach" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;

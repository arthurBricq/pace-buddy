import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider } from './hooks/useAuth';
import AuthGuard from './components/AuthGuard';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import ActivityListPage from './pages/ActivityListPage';
import ActivityDetailPage from './pages/ActivityDetailPage';
import TrainingsListPage from './pages/TrainingsListPage';
import TrainingDetailPage from './pages/TrainingDetailPage';
import RacesPage from './pages/RacesPage';
import ProfilePage from './pages/ProfilePage';
import AiChatsListPage from './pages/AiChatsListPage';
import AiChatPage from './pages/AiChatPage';
import AdminDashboardPage from './pages/AdminDashboardPage';

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/register" element={<RegisterPage />} />
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
            path="/chats"
            element={
              <AuthGuard>
                <AiChatsListPage />
              </AuthGuard>
            }
          />
          <Route
            path="/chats/:id"
            element={
              <AuthGuard>
                <AiChatPage />
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
                <AdminDashboardPage />
              </AuthGuard>
            }
          />
          <Route path="*" element={<Navigate to="/activities" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;

import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AuthProvider } from './hooks/useAuth';
import AuthGuard from './components/AuthGuard';
import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';
import ActivityListPage from './pages/ActivityListPage';
import ActivityDetailPage from './pages/ActivityDetailPage';
import LinkStravaPage from './pages/LinkStravaPage';
import TrainingsListPage from './pages/TrainingsListPage';
import TrainingDetailPage from './pages/TrainingDetailPage';

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
          <Route
            path="/strava"
            element={
              <AuthGuard>
                <LinkStravaPage />
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
          <Route path="*" element={<Navigate to="/activities" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;

import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';

export default function Navbar() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  if (!user) return null;

  return (
    <nav className="bg-white border-b border-gray-200 px-6 py-3 flex items-center justify-between">
      <div className="flex items-center gap-6">
        <Link to="/activities" className="text-lg font-bold text-gray-900">
          RunningTool
        </Link>
        <Link to="/activities" className="text-sm text-gray-600 hover:text-gray-900">
          Activities
        </Link>
        <Link to="/trainings" className="text-sm text-gray-600 hover:text-gray-900">
          Trainings
        </Link>
        <Link to="/strava" className="text-sm text-gray-600 hover:text-gray-900">
          Strava
        </Link>
      </div>
      <div className="flex items-center gap-4">
        <span className="text-sm text-gray-500">{user.display_name}</span>
        <button
          onClick={handleLogout}
          className="text-sm text-gray-500 hover:text-gray-700"
        >
          Logout
        </button>
      </div>
    </nav>
  );
}

import { Link, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';

function NavLink({ to, children }: { to: string; children: React.ReactNode }) {
  const { pathname } = useLocation();
  const active = pathname === to || pathname.startsWith(to + '/');
  return (
    <Link
      to={to}
      className={`text-sm ${active ? 'text-gray-900 font-semibold' : 'text-gray-500 hover:text-gray-900'}`}
    >
      {children}
    </Link>
  );
}

export default function Navbar() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  if (!user) return null;

  return (
    <nav className="sticky top-0 z-50 bg-white border-b border-gray-200 px-6 py-3 flex items-center justify-between">
      <div className="flex items-center gap-6">
        <Link to="/activities" className="text-lg font-bold text-gray-900">
          RunningTool
        </Link>
        <NavLink to="/activities">Activities</NavLink>
        <NavLink to="/trainings">Trainings</NavLink>
        <NavLink to="/chats">AI Chats</NavLink>
        <NavLink to="/races">Races</NavLink>
      </div>
      <div className="flex items-center gap-4">
        <NavLink to="/profile">Profile</NavLink>
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

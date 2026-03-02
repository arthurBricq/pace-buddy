import { useEffect, useState } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';

function NavLink({
  to,
  children,
  mobile = false,
  onClick,
}: {
  to: string;
  children: React.ReactNode;
  mobile?: boolean;
  onClick?: () => void;
}) {
  const { pathname } = useLocation();
  const active = pathname === to || pathname.startsWith(to + '/');
  return (
    <Link
      to={to}
      onClick={onClick}
      className={
        mobile
          ? `block rounded-md px-2 py-1 ${active ? 'bg-gray-100 text-gray-900 font-semibold' : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'}`
          : active
            ? 'nav-link-active'
            : 'nav-link'
      }
    >
      {children}
    </Link>
  );
}

export default function Navbar() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    setMenuOpen(false);
  }, [pathname]);

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  if (!user) return null;

  return (
    <nav className="navbar-root">
      <div className="navbar-inner">
        <div className="flex items-center gap-4">
          <Link to="/activities" className="navbar-brand">
            Pace Buddy
          </Link>
          <div className="navbar-links-desktop">
            <NavLink to="/activities">Activities</NavLink>
            <NavLink to="/trainings">Trainings</NavLink>
            <NavLink to="/chats">AI Chats</NavLink>
            <NavLink to="/races">Races</NavLink>
          </div>
        </div>

        <div className="navbar-actions-desktop">
          <NavLink to="/profile">Profile</NavLink>
          <button
            onClick={handleLogout}
            className="text-sm text-gray-500 hover:text-gray-700"
          >
            Logout
          </button>
        </div>

        <button
          type="button"
          className="navbar-mobile-toggle"
          aria-label="Toggle navigation menu"
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen((prev) => !prev)}
        >
          {menuOpen ? 'X' : 'Menu'}
        </button>
      </div>

      {menuOpen && (
        <div className="navbar-mobile-panel">
          <div className="navbar-mobile-links">
            <NavLink to="/activities" mobile onClick={() => setMenuOpen(false)}>
              Activities
            </NavLink>
            <NavLink to="/trainings" mobile onClick={() => setMenuOpen(false)}>
              Trainings
            </NavLink>
            <NavLink to="/chats" mobile onClick={() => setMenuOpen(false)}>
              AI Chats
            </NavLink>
            <NavLink to="/races" mobile onClick={() => setMenuOpen(false)}>
              Races
            </NavLink>
            <NavLink to="/profile" mobile onClick={() => setMenuOpen(false)}>
              Profile
            </NavLink>
            <button
              onClick={handleLogout}
              className="text-left rounded-md px-2 py-1 text-sm text-gray-600 hover:bg-gray-100 hover:text-gray-900"
            >
              Logout
            </button>
          </div>
        </div>
      )}
    </nav>
  );
}

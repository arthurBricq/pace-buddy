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
          ? active
            ? 'nav-link-mobile-active'
            : 'nav-link-mobile'
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
          <Link to="/coach" className="navbar-brand" aria-label="PaceBuddy">
            <img src="/pace-buddy-logo.svg" alt="PaceBuddy" className="navbar-brand-logo" />
          </Link>
          <div className="navbar-links-desktop">
            <NavLink to="/coach">Running Coach</NavLink>
            <NavLink to="/activities">Activities</NavLink>
            <NavLink to="/trainings">Trainings</NavLink>
            <NavLink to="/chats">AI Chats</NavLink>
            <NavLink to="/races">Races</NavLink>
          </div>
        </div>

        <div className="navbar-actions-desktop">
          <NavLink to="/help">Help</NavLink>
          <NavLink to="/profile">Profile</NavLink>
          <button
            onClick={handleLogout}
            className="navbar-logout-btn"
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
            <NavLink to="/coach" mobile onClick={() => setMenuOpen(false)}>
              Running Coach
            </NavLink>
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
            <NavLink to="/help" mobile onClick={() => setMenuOpen(false)}>
              Help
            </NavLink>
            <NavLink to="/profile" mobile onClick={() => setMenuOpen(false)}>
              Profile
            </NavLink>
            <button
              onClick={handleLogout}
              className="nav-link-mobile text-left"
            >
              Logout
            </button>
          </div>
        </div>
      )}
    </nav>
  );
}

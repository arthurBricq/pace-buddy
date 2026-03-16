import { useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { startStravaAuth } from '../api/auth';

const chatPreview = [
  'Review my last track session. What did I execute well, and what should I adjust?',
  'Is my current training block balanced for a 10K goal?',
  'Suggest a VO2max interval session for this week based on my recent load.',
  'Estimate my current MAS and how it has changed since last month.',
];

const modelGroups = [
  {
    provider: 'OpenAI',
    models: ['openai/gpt-5.4', 'openai/gpt-5.3-chat', 'openai/gpt-5.2', 'openai/gpt-5-mini'],
  },
  {
    provider: 'Anthropic',
    models: [
      'anthropic/claude-opus-4.5',
      'anthropic/claude-sonnet-4.5',
      'anthropic/claude-haiku-4.5',
      'anthropic/claude-sonnet-4',
    ],
  },
  {
    provider: 'Google',
    models: [
      'google/gemini-3.1-pro-preview',
      'google/gemini-3-pro-preview',
      'google/gemini-3-flash-preview',
      'google/gemini-2.5-flash',
      'google/gemini-2.5-flash-lite',
    ],
  },
  {
    provider: 'DeepSeek + xAI',
    models: ['deepseek/deepseek-v3.2', 'x-ai/grok-code-fast-1'],
  },
];

const coachStyles = [
  'Supportive and confidence-building',
  'Direct and no-fluff',
  'Data-first and analytical',
  'Race-focused and tactical',
];

const featureCards = [
  {
    id: 'intervals',
    title: 'Interval intelligence',
    description:
      'Automatic rep-by-rep parsing for track sessions and structured workouts, with pace, recoveries, and consistency trends.',
    points: ['Automatic interval segmentation', 'Recovery quality tracking', 'Consistency score across reps'],
  },
  {
    id: 'blocks',
    title: 'Training blocks and summaries',
    description:
      'Group sessions into training blocks and generate weekly or monthly narratives on what improved, what stalled, and what to change next.',
    points: ['Block-level progress views', 'Weekly and monthly narratives', 'Action-oriented adjustment ideas'],
  },
  {
    id: 'chat',
    title: 'Ask anything, with context',
    description:
      'Chat with a coach-style LLM grounded in your real training context, so answers stay specific and useful instead of generic.',
    points: ['Context-aware coaching answers', 'Session-aware follow-up questions', 'Clear next-step recommendations'],
  },
];

const trustItems = [
  'Athlete-only: you see your data.',
  'No AI training on your data.',
  'Disconnect anytime; data deletion supported.',
];

function FeatureIcon({ type }: { type: string }) {
  if (type === 'intervals') {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M3 15h3l2-6 4 10 3-8 2 4h4" />
      </svg>
    );
  }

  if (type === 'blocks') {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="4" y="5" width="6" height="6" rx="1.2" />
        <rect x="14" y="5" width="6" height="6" rx="1.2" />
        <rect x="4" y="13" width="6" height="6" rx="1.2" />
        <rect x="14" y="13" width="6" height="6" rx="1.2" />
      </svg>
    );
  }

  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 6h16M4 12h12M4 18h8" />
      <circle cx="19" cy="18" r="2" />
    </svg>
  );
}

export default function LoginPage() {
  const [searchParams] = useSearchParams();
  const [error, setError] = useState(searchParams.get('error') || '');
  const [stravaLoading, setStravaLoading] = useState(false);
  const [inviteCode, setInviteCode] = useState('');

  const handleStravaLogin = async () => {
    setError('');
    setStravaLoading(true);
    try {
      const code = inviteCode.trim();
      const { url } = await startStravaAuth(code || undefined);
      window.location.href = url;
    } catch (err: any) {
      setError(err.message || 'Strava login failed');
      setStravaLoading(false);
    }
  };

  return (
    <div className="brand-page">
      <header className="brand-header">
        <div className="brand-container brand-header-inner">
          <Link to="/login" className="brand-logo-lockup" aria-label="PaceBuddy home">
            <img src="/pace-buddy-logo.svg" alt="PaceBuddy" className="brand-logo-img" />
          </Link>
          <div className="brand-actions">
            <label className="brand-invite-field">
              <span>Invite code</span>
              <input
                type="text"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                placeholder="PB-XXXX-XXXX-XXXX-XXXX"
                autoComplete="one-time-code"
              />
            </label>
            <button
              type="button"
              onClick={handleStravaLogin}
              disabled={stravaLoading}
              className="brand-btn brand-strava-btn"
            >
              {stravaLoading ? (
                'Redirecting...'
              ) : (
                <img
                  src="/btn_strava_connect_with_orange.svg"
                  alt="Connect with Strava"
                  className="strava-connect-img"
                />
              )}
            </button>
          </div>
        </div>
      </header>

      <main className="brand-main">
        <section className="brand-section brand-section-compact">
          <div className="brand-container">
            <p className="hero-kicker">LLM coach + deep interval understanding</p>
            <h1 className="hero-title">Your training, explained.</h1>
            <p className="hero-body">
              Tired of copy-pasting your Strava activities into ChatGPT?
            </p>
            <p className="hero-body">
              PaceBuddy analyzes your interval sessions and
              training blocks, then lets you chat with a coach-style LLM that actually knows your context.
            </p>
            {error && <p className="brand-error">{error}</p>}
          </div>
        </section>

        <section className="brand-section brand-section-compact">
          <div className="brand-container">
            <div className="brand-panel">
              <h2 className="panel-title">Chat preview</h2>
              <p className="panel-description">
                Example context-aware prompts runners ask when training with PaceBuddy.
              </p>
              <div className="chat-preview">
                {chatPreview.map((prompt) => (
                  <article className="chat-item" key={prompt}>
                    <p className="chat-item-prompt">{prompt}</p>
                  </article>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="brand-section">
          <div className="brand-container">
            <div className="brand-panel brand-control-panel">
              <h2 className="panel-title">You control the coach</h2>
              <p className="panel-description">
                Pick the LLM that fits your needs and tune the coach voice to match how you like to receive feedback.
              </p>
              <div className="control-grid">
                <article className="control-card">
                  <h3 className="control-title">1. Choose your model</h3>
                  <p className="control-description">
                    PaceBuddy supports a curated OpenRouter model list so you can pick speed, depth, and style.
                  </p>
                  <div className="model-groups">
                    {modelGroups.map((group) => (
                      <div key={group.provider} className="model-group">
                        <p className="model-provider">{group.provider}</p>
                        <div className="model-chip-row">
                          {group.models.map((model) => (
                            <span key={model} className="model-chip">
                              {model}
                            </span>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                </article>

                <article className="control-card">
                  <h3 className="control-title">2. Shape your coach character</h3>
                  <p className="control-description">
                    Select the coaching tone you want in chat so feedback feels personal and actionable.
                  </p>
                  <div className="coach-style-row">
                    {coachStyles.map((style) => (
                      <span key={style} className="coach-style-chip">
                        {style}
                      </span>
                    ))}
                  </div>
                  <p className="coming-soon-badge">Coach character controls: coming soon</p>
                </article>
              </div>
            </div>
          </div>
        </section>

        <section className="brand-section">
          <div className="brand-container">
            <div className="features-grid">
              {featureCards.map((feature) => (
                <article className="feature-card" key={feature.title}>
                  <div className="feature-icon" aria-hidden="true">
                    <FeatureIcon type={feature.id} />
                  </div>
                  <h3 className="feature-title">{feature.title}</h3>
                  <p className="feature-description">{feature.description}</p>
                  <ul className="feature-points">
                    {feature.points.map((point) => (
                      <li key={point}>{point}</li>
                    ))}
                  </ul>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section className="brand-section">
          <div className="brand-container">
            <div className="trust-strip">
              {trustItems.map((item) => (
                <p key={item} className="trust-item">
                  {item}
                </p>
              ))}
            </div>
          </div>
        </section>
      </main>

      <footer className="brand-footer">
        <div className="brand-container brand-footer-inner">
          <img
            src="/strava_pwrdby_horiz_orange.svg"
            alt="Powered by Strava"
            className="strava-powered"
          />
        </div>
      </footer>
    </div>
  );
}

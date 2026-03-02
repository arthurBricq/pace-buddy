import Navbar from '../components/Navbar';

export default function HelpPage() {
  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-narrow section-stack">
        <div>
          <h1 className="text-2xl font-bold">Help</h1>
          <p className="text-sm text-gray-500 mt-1">
            Quick guide to understand the main concepts and get the most out of Pace Buddy.
          </p>
        </div>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">How to use the app</h2>
          <div className="space-y-4 text-sm text-gray-700">
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">What is a training?</h3>
              <p>
                A training is a time window with a goal. The app derives quality activities inside
                that range (intervals, long runs, races), then uses them for analysis and insights.
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">What is an LLM chat?</h3>
              <p>
                An LLM chat is your coaching conversation with AI. You can start from scratch, or
                continue from a training insight, and add contextual data directly in the chat.
              </p>
            </div>
          </div>
        </section>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">Tips</h2>
          <ul className="list-disc pl-5 space-y-2 text-sm text-gray-700">
            <li>
              Tag activities directly in Strava (races, workouts, long runs) so they are imported
              correctly and you avoid manual friction.
            </li>
            <li>
              Use meaningful activity names, especially for interval sessions. Clear names improve
              your own review flow and AI interpretation quality.
            </li>
          </ul>
        </section>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">About</h2>
          <p className="text-sm text-gray-700">
            Pace Buddy is a personal project under active development. Features and behavior may
            evolve quickly, so always use your own judgment for training decisions.
          </p>
        </section>

        <section className="card">
          <h2 className="text-lg font-semibold mb-3">LLM models and costs</h2>
          <p className="text-sm text-gray-700">
            Some LLM models are more expensive than others. Model cost differences are already
            reflected in usage, and cost controls will be improved further soon.
          </p>
        </section>
      </div>
    </div>
  );
}

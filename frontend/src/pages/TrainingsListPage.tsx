import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { listTrainings, createTraining, deleteTraining } from '../api/trainings';
import { errorMessage } from '../api/client';
import type { Training } from '../types';
import Navbar from '../components/Navbar';

export default function TrainingsListPage() {
  const [trainings, setTrainings] = useState<Training[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newStartDate, setNewStartDate] = useState('');
  const [newEndDate, setNewEndDate] = useState('');
  const [newRaceGoal, setNewRaceGoal] = useState('');
  const [customGoal, setCustomGoal] = useState('');
  const [newRaceObjectif, setNewRaceObjectif] = useState('');

  const load = async () => {
    setLoading(true);
    try {
      const data = await listTrainings();
      setTrainings(data);
    } catch (err: unknown) {
      setError(errorMessage(err, 'Failed to load trainings'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newName.trim()) {
      setError('Name is required');
      return;
    }
    if (!newStartDate || !newEndDate) {
      setError('Start date and end date are required');
      return;
    }
    if (new Date(newStartDate) >= new Date(newEndDate)) {
      setError('Start date must be before end date');
      return;
    }

    setCreating(true);
    setError('');
    try {
      const goalValue = newRaceGoal === 'Other' ? customGoal.trim() : newRaceGoal;
      await createTraining(
        newName.trim(),
        newDescription.trim() || undefined,
        newStartDate ? new Date(newStartDate).toISOString() : undefined,
        newEndDate ? new Date(newEndDate).toISOString() : undefined,
        goalValue || undefined,
        newRaceObjectif.trim() || undefined,
      );
      setNewName('');
      setNewDescription('');
      setNewStartDate('');
      setNewEndDate('');
      setNewRaceGoal('');
      setCustomGoal('');
      setNewRaceObjectif('');
      setShowCreateForm(false);
      load();
    } catch (err: unknown) {
      setError(errorMessage(err, 'Failed to create training'));
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Are you sure you want to delete this training?')) {
      return;
    }

    try {
      await deleteTraining(id);
      load();
    } catch (err: unknown) {
      setError(errorMessage(err, 'Failed to delete training'));
    }
  };

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide">
        <div className="page-title-row">
          <h1 className="text-xl font-bold">Trainings</h1>
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 text-sm"
          >
            {showCreateForm ? 'Cancel' : 'Create Training'}
          </button>
        </div>

        {showCreateForm && (
          <div className="card-compact mb-4">
            <form onSubmit={handleCreate}>
              <div className="mb-3">
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Name *
                </label>
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="e.g., Week 1 Intervals"
                  required
                />
              </div>
              <div className="mb-3">
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Description
                </label>
                <textarea
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="Optional description..."
                  rows={3}
                />
              </div>
              <div className="form-grid-2 mb-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Start Date *
                  </label>
                  <input
                    type="date"
                    value={newStartDate}
                    onChange={(e) => setNewStartDate(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    required
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    End Date *
                  </label>
                  <input
                    type="date"
                    value={newEndDate}
                    onChange={(e) => setNewEndDate(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    required
                  />
                </div>
              </div>
              <div className="mb-3">
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Race Distance
                </label>
                <select
                  value={newRaceGoal}
                  onChange={(e) => setNewRaceGoal(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">None</option>
                  <option value="5K">5K</option>
                  <option value="10K">10K</option>
                  <option value="Half Marathon">Half Marathon</option>
                  <option value="Marathon">Marathon</option>
                  <option value="Trail">Trail</option>
                  <option value="Ultra">Ultra</option>
                  <option value="Other">Other</option>
                </select>
                {newRaceGoal === 'Other' && (
                  <input
                    type="text"
                    value={customGoal}
                    onChange={(e) => setCustomGoal(e.target.value)}
                    className="w-full mt-2 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder="Enter custom goal..."
                  />
                )}
              </div>
              <div className="mb-3">
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Race Objectif
                </label>
                <textarea
                  value={newRaceObjectif}
                  onChange={(e) => setNewRaceObjectif(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="Describe your race objective..."
                  rows={3}
                />
              </div>
              <div className="button-row-wrap">
                <button
                  type="submit"
                  disabled={creating}
                  className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                >
                  {creating ? 'Creating...' : 'Create'}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCreateForm(false);
                    setNewName('');
                    setNewDescription('');
                    setNewStartDate('');
                    setNewEndDate('');
                    setNewRaceGoal('');
                    setCustomGoal('');
                    setNewRaceObjectif('');
                  }}
                  className="bg-gray-200 text-gray-700 px-4 py-2 rounded-md hover:bg-gray-300 text-sm"
                >
                  Cancel
                </button>
              </div>
            </form>
          </div>
        )}

        {error && <p className="text-red-600 text-sm mb-4">{error}</p>}

        {loading ? (
          <p className="text-gray-500">Loading trainings...</p>
        ) : trainings.length === 0 ? (
          <p className="text-gray-500">
            No trainings yet. Create one to get started.
          </p>
        ) : (
          <div className="data-table-wrap">
            <table className="data-table">
              <thead className="bg-gray-50 text-gray-600">
                <tr>
                  <th className="text-left px-4 py-3">Name</th>
                  <th className="text-left px-4 py-3">Description</th>
                  <th className="text-left px-4 py-3">Race Distance</th>
                  <th className="text-left px-4 py-3">Dates</th>
                  <th className="text-right px-4 py-3">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {trainings.map((t) => (
                  <tr key={t.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3">
                      <Link
                        to={`/trainings/${t.id}`}
                        className="text-blue-600 hover:underline font-medium"
                      >
                        {t.name}
                      </Link>
                    </td>
                    <td className="px-4 py-3 text-gray-500">
                      {t.description || '-'}
                    </td>
                    <td className="px-4 py-3 text-gray-500">
                      {t.race_distance || '-'}
                    </td>
                    <td className="px-4 py-3 text-gray-500 text-sm">
                      {t.start_date || t.end_date ? (
                        <>
                          {t.start_date && new Date(t.start_date).toLocaleDateString()}
                          {t.start_date && t.end_date && ' - '}
                          {t.end_date && new Date(t.end_date).toLocaleDateString()}
                        </>
                      ) : '-'}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <button
                        onClick={() => handleDelete(t.id)}
                        className="text-red-600 hover:text-red-800 text-sm"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

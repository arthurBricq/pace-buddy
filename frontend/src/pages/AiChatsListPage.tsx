import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { listChats, deleteChat, createChat } from '../api/chats';
import type { ChatListItem } from '../types';
import Navbar from '../components/Navbar';
import ChatSettingsModal from '../components/ChatSettingsModal';

export default function AiChatsListPage() {
  const navigate = useNavigate();
  const [chats, setChats] = useState<ChatListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showNewChat, setShowNewChat] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const data = await listChats();
      setChats(data);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this chat?')) return;
    try {
      await deleteChat(id);
      load();
    } catch (err: any) {
      setError(err.message);
    }
  };

  const formatCost = (cost: number) => {
    if (cost === 0) return '-';
    return `$${cost.toFixed(4)}`;
  };

  return (
    <div className="app-shell">
      <Navbar />
      <div className="page-container-wide">
        <div className="page-title-row">
          <h1 className="text-xl font-bold">AI Chats</h1>
          <button
            onClick={() => setShowNewChat(true)}
            className="bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700 text-sm"
          >
            + New Chat
          </button>
        </div>

        {error && <p className="text-red-600 text-sm mb-4">{error}</p>}

        {loading ? (
          <p className="text-gray-500">Loading chats...</p>
        ) : chats.length === 0 ? (
          <p className="text-gray-500">
            No AI chats yet. Click "+ New Chat" to start one, or generate an insight from a training.
          </p>
        ) : (
          <div className="data-table-wrap">
            <table className="data-table">
              <thead className="bg-gray-50 text-gray-600">
                <tr>
                  <th className="text-left px-4 py-3">Title</th>
                  <th className="text-left px-4 py-3">Model</th>
                  <th className="text-right px-4 py-3">Messages</th>
                  <th className="text-right px-4 py-3">Cost</th>
                  <th className="text-left px-4 py-3">Updated</th>
                  <th className="text-right px-4 py-3">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {chats.map((c) => (
                  <tr key={c.id} className="hover:bg-gray-50">
                    <td className="px-4 py-3">
                      <Link
                        to={`/chats/${c.id}`}
                        className="text-blue-600 hover:underline font-medium"
                      >
                        {c.title}
                      </Link>
                    </td>
                    <td className="px-4 py-3 text-gray-500 text-xs font-mono">
                      {c.model.split('/').pop()}
                    </td>
                    <td className="px-4 py-3 text-right text-gray-500">
                      {c.message_count}
                    </td>
                    <td className="px-4 py-3 text-right text-gray-500">
                      {formatCost(c.total_cost)}
                    </td>
                    <td className="px-4 py-3 text-gray-500">
                      {new Date(c.updated_at).toLocaleDateString(undefined, {
                        month: 'short',
                        day: 'numeric',
                        hour: '2-digit',
                        minute: '2-digit',
                      })}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <button
                        onClick={() => handleDelete(c.id)}
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
        <ChatSettingsModal
          isOpen={showNewChat}
          onClose={() => setShowNewChat(false)}
          onConfirm={async (model) => {
            try {
              const chat = await createChat('New chat', model);
              navigate(`/chats/${chat.id}`);
            } catch (err: any) {
              setError(err.message);
              setShowNewChat(false);
            }
          }}
          hideConversationLength
          title="New Chat"
          confirmLabel="Create Chat"
        />
      </div>
    </div>
  );
}

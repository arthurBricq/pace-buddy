import { useEffect, useState, useRef } from 'react';
import { useParams, Link } from 'react-router-dom';
import { getChat, sendMessage, updateChatTitle } from '../api/chats';
import type { AiChat, AiChatMessage } from '../types';
import ReactMarkdown from 'react-markdown';
import Navbar from '../components/Navbar';

export default function AiChatPage() {
  const { id } = useParams<{ id: string }>();
  const [chat, setChat] = useState<AiChat | null>(null);
  const [messages, setMessages] = useState<AiChatMessage[]>([]);
  const [totalCost, setTotalCost] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState('');
  const [input, setInput] = useState('');
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [titleValue, setTitleValue] = useState('');
  const [updatingTitle, setUpdatingTitle] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const titleInputRef = useRef<HTMLInputElement>(null);

  const loadChat = async () => {
    if (!id) return;
    try {
      const data = await getChat(id);
      setChat(data.chat);
      setMessages(data.messages);
      setTotalCost(data.total_cost);
      setTotalTokens(data.total_tokens);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadChat();
  }, [id]);

  useEffect(() => {
    if (chat) {
      setTitleValue(chat.title);
    }
  }, [chat]);

  useEffect(() => {
    if (isEditingTitle && titleInputRef.current) {
      titleInputRef.current.focus();
      titleInputRef.current.select();
    }
  }, [isEditingTitle]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = async () => {
    if (!id || !input.trim() || sending) return;
    const content = input.trim();
    setInput('');
    setSending(true);
    setError('');

    // Optimistic user message
    const optimisticMsg: AiChatMessage = {
      id: 'pending',
      chat_id: id,
      role: 'user',
      content,
      prompt_tokens: 0,
      completion_tokens: 0,
      total_tokens: 0,
      cost: 0,
      created_at: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, optimisticMsg]);

    try {
      const assistantMsg = await sendMessage(id, content);
      // Reload full chat to get accurate totals
      const data = await getChat(id);
      setMessages(data.messages);
      setTotalCost(data.total_cost);
      setTotalTokens(data.total_tokens);
    } catch (err: any) {
      setError(err.message);
      // Remove optimistic message on error
      setMessages((prev) => prev.filter((m) => m.id !== 'pending'));
    } finally {
      setSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const formatCost = (cost: number) => {
    if (cost === 0) return '$0';
    return `$${cost.toFixed(4)}`;
  };

  const handleTitleClick = () => {
    if (chat) {
      setIsEditingTitle(true);
      setTitleValue(chat.title);
    }
  };

  const handleTitleSave = async () => {
    if (!id || !chat || titleValue.trim() === chat.title) {
      setIsEditingTitle(false);
      return;
    }

    setUpdatingTitle(true);
    try {
      const updatedChat = await updateChatTitle(id, titleValue.trim());
      setChat(updatedChat);
      setIsEditingTitle(false);
    } catch (err: any) {
      setError(err.message || 'Failed to update title');
      setTitleValue(chat.title); // Revert on error
    } finally {
      setUpdatingTitle(false);
    }
  };

  const handleTitleCancel = () => {
    if (chat) {
      setTitleValue(chat.title);
    }
    setIsEditingTitle(false);
  };

  const handleTitleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleTitleSave();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      handleTitleCancel();
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-6">
          <p className="text-gray-500">Loading chat...</p>
        </div>
      </div>
    );
  }

  if (error && !chat) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-6">
          <p className="text-red-600">{error}</p>
          <Link to="/chats" className="text-blue-600 hover:underline text-sm mt-2 inline-block">
            Back to chats
          </Link>
        </div>
      </div>
    );
  }

  // Filter out system messages for display
  const visibleMessages = messages.filter((m) => m.role !== 'system');

  return (
    <div className="min-h-screen bg-gray-50 flex flex-col">
      <div className="sticky top-0 z-50">
        <Navbar />
        {/* Header bar */}
        <div className="bg-white border-b px-4 py-3">
        <div className="max-w-4xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Link to="/chats" className="text-sm text-gray-500 hover:text-gray-700">
              &larr; Chats
            </Link>
            {isEditingTitle ? (
              <input
                ref={titleInputRef}
                type="text"
                value={titleValue}
                onChange={(e) => setTitleValue(e.target.value)}
                onBlur={handleTitleSave}
                onKeyDown={handleTitleKeyDown}
                disabled={updatingTitle}
                className="text-lg font-semibold bg-white border border-purple-300 rounded px-2 py-1 focus:outline-none focus:ring-2 focus:ring-purple-500 min-w-[200px]"
              />
            ) : (
              <h1
                onClick={handleTitleClick}
                className="text-lg font-semibold cursor-pointer hover:text-purple-600 transition-colors"
                title="Click to rename"
              >
                {chat?.title}
              </h1>
            )}
          </div>
          <div className="flex items-center gap-4 text-xs text-gray-500">
            <span className="font-mono">{chat?.model.split('/').pop()}</span>
            <span>{totalTokens.toLocaleString()} tokens</span>
            <span>{formatCost(totalCost)}</span>
          </div>
        </div>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto px-4 py-4 space-y-4">
          {visibleMessages.map((msg) => (
            <div
              key={msg.id}
              className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
            >
              <div
                className={`rounded-lg px-4 py-3 max-w-[85%] ${
                  msg.role === 'user'
                    ? 'bg-purple-100 text-purple-900'
                    : 'bg-gray-100 text-gray-900'
                }`}
              >
                {msg.role === 'assistant' ? (
                  <div className="prose prose-sm max-w-none">
                    <ReactMarkdown>{msg.content}</ReactMarkdown>
                  </div>
                ) : (
                  <p className="whitespace-pre-wrap text-sm">{msg.content}</p>
                )}
                {msg.role === 'assistant' && msg.cost > 0 && (
                  <div className="mt-2 text-xs text-gray-400">
                    {msg.total_tokens} tokens &middot; {formatCost(msg.cost)}
                  </div>
                )}
              </div>
            </div>
          ))}

          {sending && (
            <div className="flex justify-start">
              <div className="bg-gray-100 rounded-lg px-4 py-3">
                <div className="flex items-center gap-2 text-gray-500 text-sm">
                  <div className="animate-spin h-4 w-4 border-2 border-purple-600 border-t-transparent rounded-full" />
                  Thinking...
                </div>
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Input */}
      <div className="bg-white border-t px-4 py-3">
        <div className="max-w-4xl mx-auto flex gap-3">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type your message... (Enter to send, Shift+Enter for newline)"
            rows={2}
            className="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 resize-none text-sm"
            disabled={sending}
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || sending}
            className="bg-purple-600 text-white px-6 py-2 rounded-md hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm self-end"
          >
            Send
          </button>
        </div>
        {error && <p className="max-w-4xl mx-auto text-red-600 text-xs mt-2">{error}</p>}
      </div>
    </div>
  );
}

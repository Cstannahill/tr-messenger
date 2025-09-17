import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Message, ConnectionStatus, ServerInfo } from '@/types/chat';

export const useChat = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({
    type: 'disconnected',
    isConnected: false,
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load messages on mount
  useEffect(() => {
    loadMessages();
  }, []);

  // Listen for new messages
  useEffect(() => {
    const unlisten = listen('message-received', (event) => {
      const newMessage = event.payload as Message;
      setMessages(prev => [...prev, newMessage]);
    });

    return () => {
      unlisten.then(unsub => unsub());
    };
  }, []);

  const loadMessages = useCallback(async () => {
    try {
      setIsLoading(true);
      const loadedMessages = await invoke<Message[]>('get_messages');
      setMessages(loadedMessages);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load messages');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const sendMessage = useCallback(async (content: string) => {
    try {
      setIsLoading(true);
      setError(null);
      const messageId = await invoke<string>('send_message', { content });
      
      // Add the message to local state immediately
      const newMessage: Message = {
        id: messageId,
        content,
        timestamp: new Date().toISOString(),
        sender: 'You',
        type: 'text',
        isRead: true,
      };
      setMessages(prev => [...prev, newMessage]);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const sendFile = useCallback(async (filePath: string) => {
    try {
      setIsLoading(true);
      setError(null);
      await invoke<string>('send_file', { filePath });
      
      // The file message will be added when we reload messages
      await loadMessages();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send file');
    } finally {
      setIsLoading(false);
    }
  }, [loadMessages]);

  const startServer = useCallback(async (port?: number) => {
    try {
      setIsLoading(true);
      setError(null);
      const serverInfo = await invoke<ServerInfo>('start_server', { port });
      
      setConnectionStatus({
        type: 'server',
        isConnected: true,
        serverInfo,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start server');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const stopServer = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      await invoke('stop_server');
      
      setConnectionStatus({
        type: 'disconnected',
        isConnected: false,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop server');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const connectToServer = useCallback(async (address: string, port: number) => {
    try {
      setIsLoading(true);
      setError(null);
      const clientInfo = await invoke<any>('connect_to_server', { address, port });
      
      setConnectionStatus({
        type: 'client',
        isConnected: true,
        clientInfo,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect to server');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const disconnect = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      await invoke('disconnect');
      
      setConnectionStatus({
        type: 'disconnected',
        isConnected: false,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to disconnect');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const discoverServers = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const servers = await invoke<ServerInfo[]>('discover_servers');
      return servers;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to discover servers');
      return [];
    } finally {
      setIsLoading(false);
    }
  }, []);

  return {
    messages,
    connectionStatus,
    isLoading,
    error,
    sendMessage,
    sendFile,
    startServer,
    stopServer,
    connectToServer,
    disconnect,
    discoverServers,
    loadMessages,
  };
};

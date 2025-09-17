export interface Message {
  id: string;
  content: string;
  timestamp: string;
  sender: string;
  type: MessageType;
  fileInfo?: FileInfo;
  isRead: boolean;
}

export interface FileInfo {
  name: string;
  size: number;
  mimeType: string;
  path?: string;
}

export type MessageType = 'text' | 'file' | 'system';

export interface ServerInfo {
  id: string;
  name: string;
  address: string;
  port: number;
  isOnline: boolean;
  lastSeen: string;
}

export interface ClientInfo {
  id: string;
  name: string;
  address: string;
  port: number;
  isConnected: boolean;
  connectedAt: string;
}

export interface ConnectionStatus {
  type: 'server' | 'client' | 'disconnected';
  isConnected: boolean;
  serverInfo?: ServerInfo;
  clientInfo?: ClientInfo;
}

export interface NetworkDiscovery {
  isDiscovering: boolean;
  discoveredServers: ServerInfo[];
}

export interface ChatState {
  messages: Message[];
  connectionStatus: ConnectionStatus;
  networkDiscovery: NetworkDiscovery;
  isLoading: boolean;
  error?: string;
}

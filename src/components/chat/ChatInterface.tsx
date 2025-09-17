import React from 'react';
import MessageList from './MessageList';
import MessageInput from './MessageInput';
import ConnectionPanel from './ConnectionPanel';
import { useChat } from '@/hooks/useChat';
import { AlertCircle, Wifi, WifiOff } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';

const ChatInterface: React.FC = () => {
  const {
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
  } = useChat();

  return (
    <div className="h-screen flex bg-background">
      {/* Connection Panel */}
      <ConnectionPanel
        connectionStatus={connectionStatus}
        onStartServer={startServer}
        onStopServer={stopServer}
        onConnectToServer={connectToServer}
        onDisconnect={disconnect}
        onDiscoverServers={discoverServers}
        isLoading={isLoading}
      />

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col">
        {/* Header */}
        <div className="border-b bg-background p-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-semibold">TCP Messenger</h1>
              <p className="text-sm text-muted-foreground">
                {connectionStatus.isConnected ? (
                  <>
                    {connectionStatus.type === 'server' ? 'Hosting server' : 'Connected to server'} • 
                    {connectionStatus.type === 'server' && connectionStatus.serverInfo && 
                      ` Port ${connectionStatus.serverInfo.port}`
                    }
                    {connectionStatus.type === 'client' && connectionStatus.clientInfo && 
                      ` ${connectionStatus.clientInfo.address}:${connectionStatus.clientInfo.port}`
                    }
                  </>
                ) : (
                  'Not connected'
                )}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {connectionStatus.isConnected ? (
                <div className="flex items-center gap-1 text-green-600">
                  <Wifi className="h-4 w-4" />
                  <span className="text-sm">Connected</span>
                </div>
              ) : (
                <div className="flex items-center gap-1 text-muted-foreground">
                  <WifiOff className="h-4 w-4" />
                  <span className="text-sm">Disconnected</span>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Error Alert */}
        {error && (
          <div className="p-4">
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          </div>
        )}

        {/* Messages */}
        <MessageList 
          messages={messages} 
          isLoading={isLoading}
        />

        {/* Message Input */}
        <MessageInput
          onSendMessage={sendMessage}
          onSendFile={sendFile}
          disabled={!connectionStatus.isConnected || isLoading}
          placeholder={
            !connectionStatus.isConnected 
              ? "Connect to a server to start messaging..."
              : "Type a message..."
          }
        />
      </div>
    </div>
  );
};

export default ChatInterface;

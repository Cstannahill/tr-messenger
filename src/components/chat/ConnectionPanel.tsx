import React, { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { 
  Server, 
  Users, 
  WifiOff, 
  Search,
  Play,
  Square
} from 'lucide-react';
import { ConnectionStatus, ServerInfo } from '@/types/chat';

interface ConnectionPanelProps {
  connectionStatus: ConnectionStatus;
  onStartServer: (port?: number) => void;
  onStopServer: () => void;
  onConnectToServer: (address: string, port: number) => void;
  onDisconnect: () => void;
  onDiscoverServers: () => Promise<ServerInfo[]>;
  isLoading: boolean;
}

const ConnectionPanel: React.FC<ConnectionPanelProps> = ({
  connectionStatus,
  onStartServer,
  onStopServer,
  onConnectToServer,
  onDisconnect,
  onDiscoverServers,
  isLoading
}) => {
  const [serverPort, setServerPort] = useState(8000);
  const [clientAddress, setClientAddress] = useState('127.0.0.1');
  const [clientPort, setClientPort] = useState(8000);
  const [discoveredServers, setDiscoveredServers] = useState<ServerInfo[]>([]);
  const [isDiscovering, setIsDiscovering] = useState(false);

  const handleDiscoverServers = async () => {
    setIsDiscovering(true);
    try {
      const servers = await onDiscoverServers();
      setDiscoveredServers(servers);
    } catch (error) {
      console.error('Failed to discover servers:', error);
    } finally {
      setIsDiscovering(false);
    }
  };

  const getConnectionBadge = () => {
    if (!connectionStatus.isConnected) {
      return <Badge variant="destructive">Disconnected</Badge>;
    }
    
    if (connectionStatus.type === 'server') {
      return <Badge variant="success">Server Running</Badge>;
    } else {
      return <Badge variant="default">Connected</Badge>;
    }
  };

  const getConnectionIcon = () => {
    if (!connectionStatus.isConnected) {
      return <WifiOff className="h-4 w-4" />;
    }
    
    if (connectionStatus.type === 'server') {
      return <Server className="h-4 w-4" />;
    } else {
      return <Users className="h-4 w-4" />;
    }
  };

  return (
    <div className="w-80 border-r bg-muted/50 flex flex-col">
      <div className="p-4 border-b">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">Connection</h2>
          <div className="flex items-center gap-2">
            {getConnectionIcon()}
            {getConnectionBadge()}
          </div>
        </div>

        {connectionStatus.isConnected && (
          <div className="text-sm text-muted-foreground">
            {connectionStatus.type === 'server' && connectionStatus.serverInfo && (
              <div>
                <div>Server: {connectionStatus.serverInfo.name}</div>
                <div>Port: {connectionStatus.serverInfo.port}</div>
                <div>Address: {connectionStatus.serverInfo.address}</div>
              </div>
            )}
            {connectionStatus.type === 'client' && connectionStatus.clientInfo && (
              <div>
                <div>Connected to: {connectionStatus.clientInfo.address}</div>
                <div>Port: {connectionStatus.clientInfo.port}</div>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="flex-1 p-4 space-y-4">
        {/* Server Controls */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              <Server className="h-4 w-4" />
              Start Server
            </CardTitle>
            <CardDescription>
              Host a server for others to connect
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex gap-2">
              <Input
                type="number"
                placeholder="Port"
                value={serverPort}
                onChange={(e) => setServerPort(Number(e.target.value))}
                disabled={isLoading || connectionStatus.isConnected}
                className="flex-1"
              />
              {connectionStatus.type === 'server' ? (
                <Button
                  onClick={onStopServer}
                  disabled={isLoading}
                  variant="destructive"
                  size="icon"
                >
                  <Square className="h-4 w-4" />
                </Button>
              ) : (
                <Button
                  onClick={() => onStartServer(serverPort)}
                  disabled={isLoading || connectionStatus.isConnected}
                  size="icon"
                >
                  <Play className="h-4 w-4" />
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Client Controls */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              <Users className="h-4 w-4" />
              Connect to Server
            </CardTitle>
            <CardDescription>
              Connect to an existing server
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="space-y-2">
              <Input
                placeholder="Server Address"
                value={clientAddress}
                onChange={(e) => setClientAddress(e.target.value)}
                disabled={isLoading || connectionStatus.isConnected}
              />
              <div className="flex gap-2">
                <Input
                  type="number"
                  placeholder="Port"
                  value={clientPort}
                  onChange={(e) => setClientPort(Number(e.target.value))}
                  disabled={isLoading || connectionStatus.isConnected}
                  className="flex-1"
                />
                {connectionStatus.type === 'client' ? (
                  <Button
                    onClick={onDisconnect}
                    disabled={isLoading}
                    variant="destructive"
                    size="icon"
                  >
                    <Square className="h-4 w-4" />
                  </Button>
                ) : (
                  <Button
                    onClick={() => onConnectToServer(clientAddress, clientPort)}
                    disabled={isLoading || connectionStatus.isConnected}
                    size="icon"
                  >
                    <Play className="h-4 w-4" />
                  </Button>
                )}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Network Discovery */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              <Search className="h-4 w-4" />
              Discover Servers
            </CardTitle>
            <CardDescription>
              Find servers on your local network
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <Button
              onClick={handleDiscoverServers}
              disabled={isLoading || isDiscovering}
              variant="outline"
              className="w-full"
            >
              {isDiscovering ? (
                <>
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-current mr-2"></div>
                  Discovering...
                </>
              ) : (
                <>
                  <Search className="h-4 w-4 mr-2" />
                  Discover Servers
                </>
              )}
            </Button>

            {discoveredServers.length > 0 && (
              <div className="space-y-2">
                <Separator />
                <div className="text-sm font-medium">Found Servers:</div>
                {discoveredServers.map((server) => (
                  <div
                    key={server.id}
                    className="p-2 bg-background rounded border flex items-center justify-between"
                  >
                    <div>
                      <div className="font-medium text-sm">{server.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {server.address}:{server.port}
                      </div>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => onConnectToServer(server.address, server.port)}
                      disabled={isLoading || connectionStatus.isConnected}
                    >
                      Connect
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
};

export default ConnectionPanel;

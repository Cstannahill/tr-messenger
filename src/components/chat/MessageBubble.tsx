import React from 'react';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Message, MessageType } from '@/types/chat';
import { FileIcon, UserIcon, BotIcon } from 'lucide-react';

interface MessageBubbleProps {
  message: Message;
  isOwn: boolean;
}

const MessageBubble: React.FC<MessageBubbleProps> = ({ message, isOwn }) => {
  const formatTime = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString([], { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
  };

  const getMessageIcon = (type: MessageType) => {
    switch (type) {
      case 'file':
        return <FileIcon className="h-4 w-4" />;
      case 'system':
        return <BotIcon className="h-4 w-4" />;
      default:
        return <UserIcon className="h-4 w-4" />;
    }
  };

  const getMessageVariant = (type: MessageType) => {
    switch (type) {
      case 'system':
        return 'secondary';
      case 'file':
        return 'outline';
      default:
        return 'default';
    }
  };

  if (message.type === 'system') {
    return (
      <div className="flex justify-center my-2">
        <div className="bg-muted px-3 py-1 rounded-full text-sm text-muted-foreground flex items-center gap-2">
          {getMessageIcon(message.type)}
          {message.content}
        </div>
      </div>
    );
  }

  return (
    <div className={cn(
      "flex mb-4",
      isOwn ? "justify-end" : "justify-start"
    )}>
      <div className={cn(
        "max-w-[70%] rounded-lg px-4 py-2",
        isOwn 
          ? "bg-primary text-primary-foreground" 
          : "bg-muted text-muted-foreground"
      )}>
        {!isOwn && (
          <div className="flex items-center gap-2 mb-1">
            <Badge variant={getMessageVariant(message.type)} className="text-xs">
              {getMessageIcon(message.type)}
              <span className="ml-1">{message.sender}</span>
            </Badge>
          </div>
        )}
        
        <div className="whitespace-pre-wrap break-words">
          {message.type === 'file' && message.fileInfo ? (
            <div className="flex items-center gap-2">
              <FileIcon className="h-4 w-4" />
              <div>
                <div className="font-medium">{message.fileInfo.name}</div>
                <div className="text-xs opacity-75">
                  {(message.fileInfo.size / 1024).toFixed(1)} KB
                </div>
              </div>
            </div>
          ) : (
            message.content
          )}
        </div>
        
        <div className={cn(
          "text-xs mt-1",
          isOwn ? "text-primary-foreground/70" : "text-muted-foreground/70"
        )}>
          {formatTime(message.timestamp)}
        </div>
      </div>
    </div>
  );
};

export default MessageBubble;

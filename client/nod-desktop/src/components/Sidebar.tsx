import { Bell, RefreshCw, Server, Settings } from "lucide-react";
import { pendingCountFor, totalPendingCount } from "../domain";
import type { Channel, ClientState, ServerProfile } from "../types";

interface SidebarProps {
  activeChannel?: Channel;
  onOpenSettings: () => void;
  onRefresh: () => Promise<void>;
  onSelectChannel: (channel: Channel) => Promise<void>;
  onSelectServer: (server: ServerProfile) => Promise<void>;
  state: ClientState;
}

export function Sidebar({
  activeChannel,
  onOpenSettings,
  onRefresh,
  onSelectChannel,
  onSelectServer,
  state,
}: SidebarProps): JSX.Element {
  return (
    <aside className="sidebar">
      <div className="brand">
        <Bell size={20} />
        <span>Nod</span>
        <strong>{totalPendingCount(state)}</strong>
      </div>
      <ServerList
        servers={state.servers}
        selectedServerId={state.selected_server_id ?? null}
        onSelect={onSelectServer}
      />
      <ChannelList
        channels={state.channels.filter((channel) => channel.subscribed)}
        activeChannel={activeChannel}
        state={state}
        onSelect={onSelectChannel}
      />
      <div className="sidebarActions">
        <button type="button" onClick={() => void onRefresh()}>
          <RefreshCw size={16} />
          Refresh
        </button>
        <button type="button" onClick={onOpenSettings}>
          <Settings size={16} />
          Settings
        </button>
      </div>
    </aside>
  );
}

interface ServerListProps {
  onSelect: (server: ServerProfile) => Promise<void>;
  selectedServerId: string | null;
  servers: ServerProfile[];
}

function ServerList({
  onSelect,
  selectedServerId,
  servers,
}: ServerListProps): JSX.Element {
  return (
    <section className="navGroup">
      <h2>Servers</h2>
      {servers.map((server) => (
        <button
          type="button"
          key={server.id}
          className={server.id === selectedServerId ? "active" : ""}
          onClick={() => void onSelect(server)}
        >
          <Server size={16} />
          <span>{server.name}</span>
        </button>
      ))}
    </section>
  );
}

interface ChannelListProps {
  activeChannel?: Channel;
  channels: Channel[];
  onSelect: (channel: Channel) => Promise<void>;
  state: ClientState;
}

function ChannelList({
  activeChannel,
  channels,
  onSelect,
  state,
}: ChannelListProps): JSX.Element {
  return (
    <section className="navGroup channels">
      <h2>Channels</h2>
      {channels.map((channel) => (
        <button
          type="button"
          key={channel.id}
          className={channel.id === activeChannel?.id ? "active" : ""}
          onClick={() => void onSelect(channel)}
        >
          <span className="swatch" style={{ backgroundColor: channel.color }} />
          <span>{channel.name}</span>
          <strong>{pendingCountFor(channel, state)}</strong>
        </button>
      ))}
    </section>
  );
}

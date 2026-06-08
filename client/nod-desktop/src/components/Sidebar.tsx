import { Bell, RefreshCw, Server, Settings } from "lucide-react";
import { pendingCountFor, totalPendingCount } from "../domain";
import type { Source, ClientState, ServerProfile } from "../types";

interface SidebarProps {
  activeSource?: Source;
  onOpenSettings: () => void;
  onRefresh: () => Promise<void>;
  onSelectSource: (source: Source) => Promise<void>;
  onSelectServer: (server: ServerProfile) => Promise<void>;
  state: ClientState;
}

export function Sidebar({
  activeSource,
  onOpenSettings,
  onRefresh,
  onSelectSource,
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
      <SourceList
        sources={state.sources.filter((source) => source.subscribed)}
        activeSource={activeSource}
        state={state}
        onSelect={onSelectSource}
      />
      <div className="sidebarControls">
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

interface SourceListProps {
  activeSource?: Source;
  sources: Source[];
  onSelect: (source: Source) => Promise<void>;
  state: ClientState;
}

function SourceList({
  activeSource,
  sources,
  onSelect,
  state,
}: SourceListProps): JSX.Element {
  return (
    <section className="navGroup sources">
      <h2>Sources</h2>
      {sources.map((source) => (
        <button
          type="button"
          key={source.id}
          className={source.id === activeSource?.id ? "active" : ""}
          onClick={() => void onSelect(source)}
        >
          <span className="swatch" style={{ backgroundColor: source.color }} />
          <span>{source.name}</span>
          <strong>{pendingCountFor(source, state)}</strong>
        </button>
      ))}
    </section>
  );
}

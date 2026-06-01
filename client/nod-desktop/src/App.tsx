import { useDesktopClient } from "./app/useDesktopClient";
import { EnrollmentView } from "./components/EnrollmentView";
import { EventDetail } from "./components/EventDetail";
import { EventList } from "./components/EventList";
import { SettingsDialog } from "./components/SettingsDialog";
import { Sidebar } from "./components/Sidebar";
import { Topbar } from "./components/Topbar";

export function App(): JSX.Element {
  const client = useDesktopClient();

  if (client.isLoading) {
    return <div className="boot">Nod</div>;
  }

  if (!client.state.is_registered) {
    return (
      <EnrollmentView
        error={client.error}
        onEnroll={client.actions.enrollDevice}
      />
    );
  }

  return (
    <div className="shell">
      <Sidebar
        activeChannel={client.activeChannel}
        onOpenSettings={client.actions.openSettings}
        onRefresh={client.actions.refreshState}
        onSelectChannel={client.actions.selectChannel}
        onSelectServer={client.actions.selectServer}
        state={client.state}
      />
      <main className="workbench">
        <Topbar
          activeChannel={client.activeChannel}
          error={client.error}
          isConnected={client.state.is_sync_connected}
          onDismissError={client.actions.clearError}
        />
        <section className="columns">
          <EventList
            events={client.state.events}
            selectedEventId={client.activeEvent?.id ?? null}
            onSelect={client.actions.selectEvent}
          />
          <EventDetail
            event={client.activeEvent}
            onAction={client.actions.submitEventAction}
            onOpenUrl={client.actions.openUrl}
          />
        </section>
      </main>
      {client.settingsOpen ? (
        <SettingsDialog
          actions={client.actions}
          devices={client.devices}
          state={client.state}
        />
      ) : null}
    </div>
  );
}

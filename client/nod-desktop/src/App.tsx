import { useDesktopClient } from "./app/useDesktopClient";
import { EnrollmentView } from "./components/EnrollmentView";
import { RequestDetail } from "./components/RequestDetail";
import { RequestList } from "./components/RequestList";
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
        onEnroll={client.commands.enrollDevice}
      />
    );
  }

  return (
    <div className="shell">
      <Sidebar
        activeChannel={client.activeChannel}
        onOpenSettings={client.commands.openSettings}
        onRefresh={client.commands.refreshState}
        onSelectChannel={client.commands.selectChannel}
        onSelectServer={client.commands.selectServer}
        state={client.state}
      />
      <main className="workbench">
        <Topbar
          activeChannel={client.activeChannel}
          error={client.error}
          isConnected={client.state.is_sync_connected}
          onDismissError={client.commands.clearError}
        />
        <section className="columns">
          <RequestList
            key={client.activeChannel?.id ?? "all"}
            requests={client.state.requests}
            selectedRequestId={client.activeRequest?.id ?? null}
            onSelect={client.commands.selectRequest}
          />
          <RequestDetail
            request={client.activeRequest}
            onOption={client.commands.submitRequestOption}
            onOpenUrl={client.commands.openUrl}
          />
        </section>
      </main>
      {client.settingsOpen ? (
        <SettingsDialog
          commands={client.commands}
          devices={client.devices}
          state={client.state}
        />
      ) : null}
    </div>
  );
}

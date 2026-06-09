import type { Channel } from "../../types";

interface ChannelSubscriptionsProps {
  channels: Channel[];
  onToggleChannel: (channel: Channel) => Promise<void>;
}

export function ChannelSubscriptions({
  channels,
  onToggleChannel,
}: ChannelSubscriptionsProps): JSX.Element {
  return (
    <section className="settingsSection">
      <h3>Channels</h3>
      {channels.map((channel) => (
        <label className="checkRow" key={channel.id}>
          <input
            type="checkbox"
            checked={channel.subscribed}
            onChange={() => void onToggleChannel(channel)}
          />
          <span aria-hidden="true">{channel.emoji || "🔔"}</span>
          <span>{channel.name}</span>
        </label>
      ))}
    </section>
  );
}

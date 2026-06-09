import NodKit
import SwiftUI

struct SubscriptionSheet: View {
  @Environment(\.dismiss) private var dismiss
  @EnvironmentObject private var store: NodStore

  var body: some View {
    NavigationStack {
      Form {
        Section("Notification Sound") {
          Picker("Sound", selection: Binding(
            get: { store.notificationSound },
            set: { sound in
              Task {
                await store.setNotificationSound(sound)
              }
            }
          )) {
            ForEach(NodStore.notificationSoundOptions) { option in
              Text(option.label).tag(option.id)
            }
          }
          #if os(macOS)
          Button {
            Task { await store.requestAndTestNotifications() }
          } label: {
            Label("Request/Test Notifications", systemImage: "bell.badge")
          }
          Button {
            openNodNotificationSettings()
          } label: {
            Label("Open Notification Settings", systemImage: "gear")
          }
          #endif
        }

        Section("Channels") {
          ForEach(store.channels) { channel in
            Toggle(isOn: Binding(
              get: { store.channels.first(where: { $0.id == channel.id })?.subscribed ?? false },
              set: { subscribed in
                Task {
                  await store.setSubscription(channelId: channel.id, subscribed: subscribed)
                }
              }
            )) {
              ChannelLabel(channel: channel)
            }
          }
        }
      }
      .navigationTitle("Subscriptions")
      .toolbar {
        ToolbarItem(placement: .confirmationAction) {
          Button("Done") {
            dismiss()
          }
        }
      }
    }
  }
}

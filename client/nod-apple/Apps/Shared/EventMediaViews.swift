import NodKit
import SwiftUI

#if os(iOS) || os(macOS)
@preconcurrency import LinkPresentation
#endif

func resolvedImageURL(_ value: String?) -> URL? {
  guard let value else {
    return nil
  }
  return normalizedWebURL(from: value)
}

func resolvedLink(_ link: NodLink) -> (label: String, url: URL)? {
  let rawURL = link.url.trimmingCharacters(in: .whitespacesAndNewlines)
  guard let url = normalizedWebURL(from: rawURL) else {
    return nil
  }

  let rawLabel = link.label.trimmingCharacters(in: .whitespacesAndNewlines)
  let label = rawLabel == "https" && rawURL.hasPrefix("//") ? linkLabel(from: url) : rawLabel
  return (label.isEmpty ? linkLabel(from: url) : label, url)
}

private func linkLabel(from url: URL) -> String {
  let path = url.path == "/" ? "" : url.path
  if let host = url.host, !host.isEmpty {
    return host + path
  }
  return url.absoluteString
}

private func normalizedWebURL(from value: String) -> URL? {
  let rawURL = value.trimmingCharacters(in: .whitespacesAndNewlines)
  let normalizedURL: String
  if rawURL.hasPrefix("//") {
    normalizedURL = "https:" + rawURL
  } else if rawURL.lowercased().hasPrefix("www.") {
    normalizedURL = "https://" + rawURL
  } else {
    normalizedURL = rawURL
  }

  guard let url = URL(string: normalizedURL),
    let scheme = url.scheme?.lowercased(),
    ["http", "https"].contains(scheme)
  else {
    return nil
  }
  return url
}

struct EventImageView: View {
  let url: URL

  var body: some View {
    AsyncImage(url: url, transaction: Transaction(animation: .default)) { phase in
      switch phase {
      case .empty:
        ProgressView()
          .frame(maxWidth: .infinity, minHeight: 180)
          .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
      case .success(let image):
        image
          .resizable()
          .scaledToFit()
          .frame(maxWidth: .infinity, maxHeight: 420)
          .background(.quaternary)
          .clipShape(RoundedRectangle(cornerRadius: 8))
      case .failure:
        Link(destination: url) {
          Label("Open Image", systemImage: "photo")
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(12)
        }
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
      @unknown default:
        EmptyView()
      }
    }
  }
}

struct EventLinkView: View {
  let label: String
  let url: URL

  var body: some View {
    #if os(iOS) || os(macOS)
      LinkPreviewCard(label: label, url: url)
    #else
      Link(destination: url) {
        Label(label, systemImage: "arrow.up.right.square")
      }
    #endif
  }
}

#if os(iOS) || os(macOS)
private struct LinkPreviewCard: View {
  @Environment(\.openURL) private var openURL
  @StateObject private var model = LinkPreviewModel()
  let label: String
  let url: URL

  var body: some View {
    Button {
      openURL(url)
    } label: {
      Group {
        if let metadata = model.metadata {
          LinkPreviewRepresentable(metadata: metadata)
            .frame(height: 96)
            .clipShape(RoundedRectangle(cornerRadius: 8))
        } else {
          HStack(spacing: 10) {
            Image(systemName: "link")
              .font(.headline)
              .frame(width: 28, height: 28)
              .foregroundStyle(.tint)
            VStack(alignment: .leading, spacing: 2) {
              Text(label)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.primary)
                .lineLimit(2)
              Text(url.host ?? url.absoluteString)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
            }
            Spacer(minLength: 8)
            Image(systemName: "arrow.up.right.square")
              .foregroundStyle(.secondary)
          }
          .padding(12)
          .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .buttonStyle(.plain)
    .task(id: url) {
      model.load(url: url)
    }
  }
}

@MainActor
private final class LinkPreviewModel: ObservableObject {
  @Published var metadata: LPLinkMetadata?
  private var requestedURL: URL?
  private var provider: LPMetadataProvider?

  func load(url: URL) {
    guard requestedURL != url else {
      return
    }
    requestedURL = url
    metadata = nil

    let provider = LPMetadataProvider()
    self.provider = provider
    provider.startFetchingMetadata(for: url) { [weak self] metadata, _ in
      let result = LinkMetadataResult(metadata: metadata)
      Task { @MainActor [weak self, result] in
        guard let self, self.requestedURL == url else {
          return
        }
        self.metadata = result.metadata
        self.provider = nil
      }
    }
  }
}

private struct LinkMetadataResult: @unchecked Sendable {
  let metadata: LPLinkMetadata?
}

#if os(iOS)
private struct LinkPreviewRepresentable: UIViewRepresentable {
  let metadata: LPLinkMetadata

  func makeUIView(context: Context) -> LPLinkView {
    LPLinkView(metadata: metadata)
  }

  func updateUIView(_ uiView: LPLinkView, context: Context) {
    uiView.metadata = metadata
  }
}
#elseif os(macOS)
private struct LinkPreviewRepresentable: NSViewRepresentable {
  let metadata: LPLinkMetadata

  func makeNSView(context: Context) -> LPLinkView {
    LPLinkView(metadata: metadata)
  }

  func updateNSView(_ nsView: LPLinkView, context: Context) {
    nsView.metadata = metadata
  }
}
#endif
#endif

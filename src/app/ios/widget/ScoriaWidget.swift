//
//  ScoriaWidget.swift
//  ScoriaWidget
//

import WidgetKit
import SwiftUI

struct Provider: TimelineProvider {
    func placeholder(in context: Context) -> SimpleEntry {
        SimpleEntry(date: Date(), is_on: true)
    }

    func getSnapshot(in context: Context, completion: @escaping (SimpleEntry) -> ()) {
        let entry = SimpleEntry(date: Date(), is_on: true)
        completion(entry)
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<Entry>) -> ()) {
        let is_on = UserDefaults(suiteName: "group.com.aperturebeam.epsilon")!.object(forKey: "location_on") as? Bool ?? false
        let entry = SimpleEntry(date: Date(), is_on: is_on)
        // update once right now, then don't get a new timeline since the app triggers the updates
        let timeline = Timeline(entries: [entry], policy: .never)
        completion(timeline)
    }
}

struct SimpleEntry: TimelineEntry {
    let date: Date
    let is_on: Bool
}

struct ScoriaWidgetEntryView : View {
    var entry: Provider.Entry
    
    @Environment(\.widgetFamily) var widgetFamily

    var body: some View {
        switch widgetFamily {
        case .accessoryInline:
            Label {
                if entry.is_on {
                    Text("On")
                } else {
                    Text("Off")
                }
            } icon : {
                // Styling the icon doesn't work in the .accessoryInline widget, so we use a version of
                // the icon that is padded on the top and bottom, and less on the sides, to fit in
                // with the text better.
                // Wrapping the image in this funning way is needed for some reason
                Image(uiImage: UIImage(named: "compass_white_inline_pad_opt") ?? UIImage())
            }
        case .accessoryCircular:
            if entry.is_on {
                // Full bright icon
                Image("compass_white_opt").resizable().frame(width: 60.0, height: 60.0)
            } else {
                // Half bright icon with x circle in the corner
                ZStack(alignment: .topTrailing) {
                    Image("compass_white_opt").resizable().frame(width: 60.0, height: 60.0).opacity(0.5)
                    Image(systemName: "multiply.circle.fill").padding(10)
                }
            }
        default:
            Text("Not available")
        }
    }
}

struct ScoriaWidget: Widget {
    let kind: String = "ScoriaWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: Provider()) { entry in
            if #available(iOS 17.0, *) {
                ScoriaWidgetEntryView(entry: entry)
                    .containerBackground(.fill.tertiary, for: .widget)
            } else {
                ScoriaWidgetEntryView(entry: entry)
                    .padding()
                    .background()
            }
        }
        .configurationDisplayName("Scoria Status")
        .description("Shows if logging is on")
        .supportedFamilies(
                    [
                        .accessoryInline,
                        .accessoryCircular,
                    ]
                )
    }
}

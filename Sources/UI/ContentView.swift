import RustLib
import SwiftUI

public struct ContentView: View {
    public init() {}
    
    public var body: some View {
        TabView {
            SenseView()
                .tabItem {
                    Label("Sense", systemImage: "waveform")
                }
            AnalyzeView()
                .tabItem {
                    Label("Analyze", systemImage: "chart.xyaxis.line")
                }
            Text("Settings view here")
                .tabItem {
                    Label("Settings", systemImage: "gear")
                }
        }.environmentObject(MyLocationManager())
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
/*
private class ResourceHandle {}

extension Bundle {
    static let resources = Bundle(for: ResourceHandle.self)
}
*/

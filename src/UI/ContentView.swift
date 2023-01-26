import RustLib
import SwiftUI
import RustLib

public struct ContentView: View {
    public init() {
        // Set the documents directory known to the core library
        SetDocumentsDir(getDocumentsDirectory().path())
    }
    
    @StateObject var myLocationManager = MyLocationManager()
    
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
        }
        .environmentObject(myLocationManager)
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

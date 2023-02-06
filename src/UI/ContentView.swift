import RustLib
import SwiftUI

public struct ContentView: View {
    public init() {
        // Set the documents directory known to the core library
        set_app_dirs(getDocumentsDirectory().path(),
                     getLibraryDirectory().path(),
                     getTemporaryDirectoryPath(),
                     getBundlePath())
        // TODO: also set other directories
        // getLibraryDirectory()  // persistent non-user data
        // getTemporaryDirectory()  // temporarily cached data
        // getBundlePath()  // bundle directory for things bundled with the app
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

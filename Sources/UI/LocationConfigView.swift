import Foundation
import SwiftUI
import CoreLocation

struct LocationConfigView: View {
    
    @EnvironmentObject var myLocationManager: MyLocationManager
  
    var body: some View {
        VStack {
            HStack {
                Image(systemName: "globe")
                Text("Location")
                    .font(.title)
            }
            .foregroundColor(.accentColor)
            .padding(.bottom, 20)
            Text("Current Location:")
                .font(.headline)
            Text("xx, yy")
                .padding(.bottom, 20)
            Text("Num data points this hour: xx")
            Text("xx updates/minute")
                .padding(.bottom, 20)
            Button(action: shareLocationLog) {
              Text("Share log")
            }
        }
    }
    func shareLocationLog() {
        shareFile(file: myLocationManager.logURL)
    }
}

struct LocationConfigView_Previews: PreviewProvider {
    static var previews: some View {
        LocationConfigView()
            .environmentObject(MyLocationManager())
    }
}

// SHARE SHEET
func shareFile(file: URL) {
    var filesToShare = [Any]()  // Create the Array which includes the files to share
    filesToShare.append(file)
    // Make the activityViewContoller which shows the share-view
    let activityViewController = UIActivityViewController(activityItems: filesToShare, applicationActivities: nil)
    
    // Show the share-view
    //UIApplication.shared.windows.first?.rootViewController?.present(activityViewController, animated: true, completion: nil)
    // Get the first window from the connectedScenes object
    if let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene {
        // Get the root view controller of the first window
        let rootViewController = windowScene.windows.first?.rootViewController
        
        // Present the share sheet to the user
        rootViewController?.present(activityViewController, animated: true, completion: nil)
    }
}

func getMinutesDecimal() -> Float {
    let mins = Float(Calendar.current.component(.minute, from: Date()))
    let secs = Float(Calendar.current.component(.second, from: Date()))
    return mins + secs / 60
}

func getDocumentsDirectory() -> URL {
    // find all possible documents directories for this user
    let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
    
    // just send back the first one, which ought to be the only one
    return paths[0]
}

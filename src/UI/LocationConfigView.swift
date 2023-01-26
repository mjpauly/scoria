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
            Text("\(myLocationManager.currentLocation.coordinate.latitude), \(myLocationManager.currentLocation.coordinate.longitude)")
                .padding(.bottom, 20)
            Text("Num data points this hour: \(myLocationManager.updatesThisHour)")
            Text("\(String(format: "%.2f", Float(myLocationManager.updatesThisHour) / getMinutesDecimal())) updates/minute")
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


func getMinutesDecimal() -> Float {
    let mins = Float(Calendar.current.component(.minute, from: Date()))
    let secs = Float(Calendar.current.component(.second, from: Date()))
    return mins + secs / 60
}

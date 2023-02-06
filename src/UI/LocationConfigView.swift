import SwiftUI
import CoreLocation


struct LocationConfigView: View {
    
    @EnvironmentObject var myLocationManager: MyLocationManager
    @State private var distanceFilter = "5.0"
    @State private var showingFilterHelp = false
  
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
            
            HStack {
                Text("Distance Filter")
                
                TextField("Distance Filter:", text: $distanceFilter)
                    .onSubmit {
                        myLocationManager.setDistanceFilter(distance: Double(distanceFilter) ?? 1.0)
                    }
                    .padding(.leading, 100)
                    .textFieldStyle(.roundedBorder)
                
                Button("?") {
                    self.showingFilterHelp = true
                }
                .popover(isPresented: $showingFilterHelp) {
                    Text("Distance Filter: The minimum distance in meters the device must move horizontally before logging a new data point.")
                        .padding(.leading, 30).padding(.trailing, 30)
                }
            }
            .padding(.bottom, 20)
            
            Button(action: shareTextLocationLog) {
              Text("Share text log")
            }.buttonStyle(.bordered)
            Button(action: shareSQLiteLocationLog) {
              Text("Share SQLite log")
            }.buttonStyle(.bordered)
        }
        .padding([.leading, .trailing], 30)
    }
    func shareTextLocationLog() {
        shareFile(file: myLocationManager.logURL)
    }
    func shareSQLiteLocationLog() {
        shareFile(file: myLocationManager.sqlURL)
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

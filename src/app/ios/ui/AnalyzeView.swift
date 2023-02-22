import Foundation
import SwiftUI
import StemLib

struct TimeConstants {
    static let weekSeconds = 7*24*60*60.0
    static let daySeconds = 24*60*60.0
}

struct AnalyzeView: View {
    
    @State private var showWebView = false
    @State private var markerColor = Color(.sRGB, red: 1.0, green: 0.27, blue: 0.0, opacity: 0.8)
    @State private var startDate = Date(timeIntervalSinceNow: TimeInterval(-TimeConstants.weekSeconds))
    @State private var endDate = Date()
  
    var body: some View {
        VStack {
            Text("Date Range")
                .font(.headline)
            
            DatePicker("Start Time", selection: $startDate, displayedComponents: [.date, .hourAndMinute])
            DatePicker("End Time", selection: $endDate, displayedComponents: [.date, .hourAndMinute])

            HStack {
                Button(action: datesPast7Days) {
                    Text("Past 7 days")
                }.buttonStyle(.bordered)
                Button(action: datesPast24Hours) {
                    Text("Past 24 hours")
                }.buttonStyle(.bordered)
                Button(action: datesToday) {
                    Text("Today")
                }.buttonStyle(.bordered)
            }
            .padding(.bottom, 20)
            
            Text("Map Style").font(.headline)
            
            ColorPicker("Marker Appearance", selection: $markerColor)
                .padding(.bottom, 40)

//            Button(action: genViz) {
//              Text("Generate Visualization")
//            }.buttonStyle(.borderedProminent)
            
            Button {
                genViz()
                showWebView.toggle()
            } label: {
                Text("Show Visualization")
            }.buttonStyle(.borderedProminent)
            .sheet(isPresented: $showWebView) {
                //WebView(url: getDocumentsDirectory().appendingPathComponent("viz.html"))
                WebView(url: URL(string: "http://127.0.0.1:8080") ?? getDocumentsDirectory().appendingPathComponent("viz.html"))
            }
        }
        .padding([.leading, .trailing], 30)
    }
    
//    func genWeekView() {
//        GenPastWeekViz()
//    }
    func genViz() {
        let colorComponents = $markerColor.wrappedValue.cgColor?.components
        gen_viz(
            Int(round(startDate.timeIntervalSince1970)),
            Int(round(endDate.timeIntervalSince1970)),
            Double(colorComponents?[0] ?? 1.0),
            Double(colorComponents?[1] ?? 0.27),
            Double(colorComponents?[2] ?? 0.0),
            Double(colorComponents?[3] ?? 0.8)
        )
    }
    
    func datesPastSeconds(secs: Double) {
        endDate = Date()
        startDate = Date(timeIntervalSinceNow: -secs)
    }
    func datesPast7Days() {
        datesPastSeconds(secs: TimeConstants.weekSeconds)
    }
    func datesPast24Hours() {
        datesPastSeconds(secs: TimeConstants.daySeconds)
    }
    func datesToday() {
        endDate = Date()
        startDate = Calendar.current.startOfDay(for: Date())
    }
}

struct AnalyzeView_Previews: PreviewProvider {
    static var previews: some View {
        AnalyzeView()
    }
}

import SwiftUI
import UniformTypeIdentifiers


// A DocumentPicker that carries metadata about what function triggered it, so
// we can call the right import handler.
public class CustomDocumentPickerViewController: UIDocumentPickerViewController {
    public var presentationSource: PresentationSource?

    convenience init(presentationSource: PresentationSource) {
        let supportedTypes: [UTType] = [UTType.data]
        self.init(forOpeningContentTypes: supportedTypes)
        self.presentationSource = presentationSource
    }
}

// The reason we're presenting a file picker.
public enum PresentationSource {
    case database
    case placesGeojson
}

// Ordinary share sheet that shares the file with its existing name
func shareFile(file: URL, viewController: UIViewController, deleteAfterShare: Bool = false) {
    // Make the activityViewContoller which shows the share-view
    let activityViewController = UIActivityViewController(activityItems: [file], applicationActivities: nil)

    // Present the sharing activity view controller
    viewController.present(activityViewController, animated: true, completion: nil)
    
    if deleteAfterShare {
        activityViewController.completionWithItemsHandler = { _, _, _, _ in
            do {
                try FileManager.default.removeItem(at: file)
            } catch {
                print_and_log_error(s: "Error removing file: \(error)")
            }
        }
    }
}

// Share sheet which renames the file (as a temporary file) and shares that.
// Falls back to the ordinary share sheet if this fails (like if there's no
// storage space on the device to create a duplicate of the log).
func shareFileWithDifferentName(originalURL: URL, desiredFilename: String, viewController: UIViewController) {
    // Create a temporary file URL with the desired filename
    let temporaryDirectory = FileManager.default.temporaryDirectory
    let temporaryURL = temporaryDirectory.appendingPathComponent(desiredFilename)
    
    do {
        // Copy the original file to the temporary location with the desired filename
        try FileManager.default.copyItem(at: originalURL, to: temporaryURL)
        
        // Create a sharing activity view controller
        let activityViewController = UIActivityViewController(activityItems: [temporaryURL], applicationActivities: nil)

        // Present the sharing activity view controller
        viewController.present(activityViewController, animated: true, completion: nil)

        // Remove the temporary file after sharing is complete
        activityViewController.completionWithItemsHandler = { _, _, _, _ in
            do {
                try FileManager.default.removeItem(at: temporaryURL)
            } catch {
                print_and_log_error(s: "Error removing temporary file: \(error)")
            }
        }
    } catch {
        print_and_log_error(s: "Error copying export file: \(error)")
        print_and_log_error(s: "Falling back to sharing the unrenamed file.")
        shareFile(file: originalURL, viewController: viewController)
    }
}

func importFile(
    viewController: MyViewControllerProtocol,
    presentationSource: PresentationSource
) {
    let documentPicker = CustomDocumentPickerViewController(
        presentationSource: presentationSource
    )
    documentPicker.delegate = viewController
    viewController.present(documentPicker, animated: true, completion: nil)
}

func getDocumentsDirectory() -> URL {
    // find all possible documents directories for this user
    let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
    
    // just send back the first one, which ought to be the only one
    return paths[0]
}

func getLibraryDirectory() -> URL {
    let paths = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask)
    return paths[0]
}

func getTemporaryDirectoryPath() -> String {
    return NSTemporaryDirectory()
}

func getBundlePath() -> String {
    return Bundle.main.bundlePath
}

import LinkPresentation
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
    case mountedDatabase
    case placesGeojson
}

// Presents an image file so the share sheet offers "Save Image" (Photos),
// which only appears for image items, not bare file URLs. Activities that
// handle files still receive the URL so the filename is preserved.
class ImageShareItem: NSObject, UIActivityItemSource {
    let file: URL
    let image: UIImage

    init?(file: URL) {
        guard let image = UIImage(contentsOfFile: file.path) else { return nil }
        self.file = file
        self.image = image
    }

    func activityViewControllerPlaceholderItem(_ activityViewController: UIActivityViewController) -> Any {
        return image
    }

    func activityViewController(_ activityViewController: UIActivityViewController, itemForActivityType activityType: UIActivity.ActivityType?) -> Any? {
        if activityType == .saveToCameraRoll {
            return image
        }
        return file
    }

    func activityViewController(_ activityViewController: UIActivityViewController, subjectForActivityType activityType: UIActivity.ActivityType?) -> String {
        return file.lastPathComponent
    }

    func activityViewController(_ activityViewController: UIActivityViewController, dataTypeIdentifierForActivityType activityType: UIActivity.ActivityType?) -> String {
        return UTType.jpeg.identifier
    }

    // Fills in the preview thumbnail and title at the top of the share sheet
    func activityViewControllerLinkMetadata(_ activityViewController: UIActivityViewController) -> LPLinkMetadata? {
        let metadata = LPLinkMetadata()
        metadata.title = file.lastPathComponent
        metadata.imageProvider = NSItemProvider(object: image)
        return metadata
    }
}

// Share sheet for an image file, including the "Save Image" activity. Falls
// back to the ordinary share sheet if the file can't be read as an image.
func shareImageFile(file: URL, viewController: UIViewController, deleteAfterShare: Bool = false) {
    guard let item = ImageShareItem(file: file) else {
        print_and_log_error(s: "Failed to load image export for sharing, sharing as a file.")
        shareFile(file: file, viewController: viewController, deleteAfterShare: deleteAfterShare)
        return
    }
    let activityViewController = UIActivityViewController(activityItems: [item], applicationActivities: nil)
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
func shareFileWithDifferentName(originalURL: URL, desiredFilename: String, viewController: UIViewController, extraFiles: [(URL, String)] = []) {
    // Copy each file to a temporary location with the desired filename
    let temporaryDirectory = FileManager.default.temporaryDirectory
    let files = [(originalURL, desiredFilename)] + extraFiles
    var temporaryURLs: [URL] = []
    do {
        for (url, name) in files {
            let temporaryURL = temporaryDirectory.appendingPathComponent(name)
            try? FileManager.default.removeItem(at: temporaryURL)
            try FileManager.default.copyItem(at: url, to: temporaryURL)
            temporaryURLs.append(temporaryURL)
        }

        // Create a sharing activity view controller
        let activityViewController = UIActivityViewController(activityItems: temporaryURLs, applicationActivities: nil)

        // Present the sharing activity view controller
        viewController.present(activityViewController, animated: true, completion: nil)

        // Remove the temporary files after sharing is complete
        activityViewController.completionWithItemsHandler = { _, _, _, _ in
            for temporaryURL in temporaryURLs {
                do {
                    try FileManager.default.removeItem(at: temporaryURL)
                } catch {
                    print_and_log_error(s: "Error removing temporary file: \(error)")
                }
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

func getTemporaryDirectory() -> URL {
    return FileManager.default.temporaryDirectory
}

func getBundlePath() -> String {
    return Bundle.main.bundlePath
}

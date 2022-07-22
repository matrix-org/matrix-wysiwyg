import XCTest
@testable import WysiwygComposer

final class WysiwygComposerTests: XCTestCase {
    func testExample() throws {
        let composer = newComposerModel()
        let update = composer.replaceText(newText: "Test")
        switch update.textUpdate() {
        case .keep:
            XCTFail()
        case .replaceAll(replacementHtml: let codeUnits,
                         startUtf16Codeunit: _,
                         endUtf16Codeunit: _):
            XCTAssertEqual(String(utf16CodeUnits: codeUnits, count: codeUnits.count),
                           "Test")
        }
    }
}

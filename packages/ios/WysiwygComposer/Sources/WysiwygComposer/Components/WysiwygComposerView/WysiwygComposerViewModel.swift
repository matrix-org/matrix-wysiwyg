//
// Copyright 2022 The Matrix.org Foundation C.I.C
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

import Foundation
import OSLog

struct WysiwygComposerViewState {
    var textSelection: NSRange
    var displayText: NSAttributedString
}

class WysiwygComposerViewModel: ObservableObject {
    @Published var viewState: WysiwygComposerViewState

    private var model: ComposerModel

    static let initialText = ""
    init() {
        self.model = newComposerModel()
        self.viewState = WysiwygComposerViewState(
            textSelection: .init(location: 0, length: 0),
            displayText: NSAttributedString()
        )
    }

    func didAttemptChange(of text: String, range: NSRange, replacementText: String) {
        let update: ComposerUpdate
        if replacementText == "" {
            update = self.model.backspace()
        } else {
            update = self.model.replaceText(newText: replacementText)
        }
        self.applyUpdate(update)
    }

    func textDidUpdate(text: String, range: NSRange) {
        // TODO if needed
    }

    func textDidChangeSelection(text: String, range: NSRange) {
        Logger.composer.debug("New selection: \(range) totalLength: \(text.count)")
        self.model.select(startUtf16Codeunit: UInt32(range.location),
                          endUtf16Codeunit: UInt32(range.location+range.length))
    }

    func applyBold() {
        let update = self.model.bold()
        self.applyUpdate(update)
    }
}

private extension WysiwygComposerViewModel {
    func applyUpdate(_ update: ComposerUpdate) {
        switch update.textUpdate() {
        case .replaceAll(replacementHtml: let codeUnits,
                         startUtf16Codeunit: let start,
                         endUtf16Codeunit: let end):
            let html = String(utf16CodeUnits: codeUnits, count: codeUnits.count).replacingOccurrences(of: "\n", with: "<br>")
            do {
                let attributed = try NSAttributedString(html: html)
                let textSelection = NSRange(location: Int(start), length: Int(end-start))
                self.viewState = WysiwygComposerViewState(
                    textSelection: textSelection,
                    displayText: attributed
                )
                Logger.composer.debug("HTML from Rust: \(html), selection: \(textSelection)")
            } catch {
                Logger.composer.error("Unable to update composer display: \(error.localizedDescription)")
            }
        default:
            break
        }
    }
}

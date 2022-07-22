// Copyright 2022 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;

use crate::owned_dom::{OwnedDom, Sink};
use crate::{ActionResponse, ComposerUpdate, Location};

fn parse_utf16(html: Vec<u16>) -> OwnedDom {
    parse_document(Sink::default(), Default::default())
        .from_utf8()
        .one(String::from_utf16(&html).unwrap().as_bytes())
}

pub struct ComposerModel<C>
where
    C: Clone,
{
    dom: OwnedDom,
    rendered: Option<Vec<C>>,
    start: Location,
    end: Location,
}

impl<C> ComposerModel<C>
where
    C: Clone,
{
    pub fn new() -> Self {
        Self {
            dom: parse_utf16(Vec::new()),
            rendered: None,
            start: Location::from(0),
            end: Location::from(0),
        }
    }

    /**
     * Cursor is at end.
     */
    pub fn select(&mut self, start: Location, end: Location) {
        self.start = start;
        self.end = end;
    }
    pub fn action_response(
        &mut self,
        action_id: String,
        response: ActionResponse,
    ) -> ComposerUpdate<C> {
        drop(action_id);
        drop(response);
        ComposerUpdate::keep()
    }

    // TODO: other functions are UTF-16-specific for now - must fix.
}

impl ComposerModel<u16> {
    fn from(html: Vec<u16>, start: usize, end: usize) -> Self {
        Self {
            dom: parse_utf16(html),
            start: Location::from(start),
            end: Location::from(end),
            rendered: None,
        }
    }

    /**
     * Return the start and end of the selection, ensuring the first number
     * returned is <= the second, and they are both 0<=n<=html.len().
     */
    fn safe_selection(&mut self) -> (usize, usize) {
        let mut s: usize = self.start.into();
        let mut e: usize = self.end.into();
        let html_len = self.html().len();
        s = s.clamp(0, html_len);
        e = e.clamp(0, html_len);
        if s > e {
            (e, s)
        } else {
            (s, e)
        }
    }

    pub fn replace_text(&mut self, new_text: &[u16]) -> ComposerUpdate<u16> {
        // TODO: escape any HTML?
        let (s, e) = self.safe_selection();
        let html = self.html();
        let mut new_html = html[..s].to_vec();
        new_html.extend_from_slice(new_text);
        new_html.extend_from_slice(&html[e..]);

        self.dom = parse_utf16(new_html);

        self.start = Location::from(s + new_text.len());
        self.end = self.start;

        // TODO: for now, we replace every time, to check ourselves, but
        // at least some of the time we should not
        self.create_update_replace_all()
        //ComposerUpdate::keep()
    }

    pub fn enter(&mut self) -> ComposerUpdate<u16> {
        ComposerUpdate::keep()
    }

    pub fn backspace(&mut self) -> ComposerUpdate<u16> {
        if self.start == self.end {
            // Go back 1 from the current location
            self.start -= 1;
        }

        self.replace_text(&[])
    }

    pub fn delete(&mut self) -> ComposerUpdate<u16> {
        if self.start == self.end {
            // Go forward 1 from the current location
            self.end += 1;
        }

        self.replace_text(&[])
    }

    pub fn bold(&mut self) -> ComposerUpdate<u16> {
        let (_s, _e) = self.safe_selection();

        // Find the node we are in
        // Check both start and end are in the same text node
        // Add a new node
        // If not in same text node, for now, refuse to do anything

        // TODO: find the node we are in. For now, guess the first one
        dbg!(&self.dom.document.children.first().unwrap().node);

        /*
        // TODO: not a real AST
        let mut new_html = self.html[..s].to_vec();
        new_html.extend("<strong>".encode_utf16().collect::<Vec<_>>());
        new_html.extend_from_slice(&self.html[s..e]);
        new_html.extend("</strong>".encode_utf16().collect::<Vec<_>>());
        new_html.extend_from_slice(&self.html[e..]);
        self.html = new_html;
        */

        /*
        TODO: probably requires a real AST
        let start_b = ByteLocation::from(range[0]);
        let end_b = ByteLocation::from(range[1] + "<strong></strong>".len());

        self.selection_start_codepoint = start_b.codepoint(&self.html);
        self.selection_end_codepoint = end_b.codepoint(&self.html);
        */

        self.create_update_replace_all()
    }

    fn html(&mut self) -> Vec<u16> {
        // TODO: hard-coded to be u16!
        if let Some(ret) = &self.rendered {
            ret.clone()
        } else {
            let s = self.dom.to_string();
            let rendered: Vec<u16> = s.encode_utf16().collect();
            let ret = rendered.clone();
            self.rendered = Some(rendered);
            ret
        }
    }

    // Internal functions

    fn create_update_replace_all(&mut self) -> ComposerUpdate<u16> {
        ComposerUpdate::replace_all(self.html().clone(), self.start, self.end)
    }
}

#[cfg(test)]
mod test {
    use speculoos::{prelude::*, AssertionFailure, Spec};

    use crate::Location;

    use super::ComposerModel;

    #[test]
    fn typing_a_character_into_an_empty_box_appends_it() {
        let mut model = cm("|");
        replace_text(&mut model, "v");
        assert_eq!(tx(&mut model), "v|");
    }

    #[test]
    fn typing_a_character_at_the_end_appends_it() {
        let mut model = cm("abc|");
        replace_text(&mut model, "d");
        assert_eq!(tx(&mut model), "abcd|");
    }

    #[test]
    fn typing_a_character_in_the_middle_inserts_it() {
        let mut model = cm("|abc");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&mut model), "Z|abc");
    }

    #[test]
    fn selecting_past_the_end_is_harmless() {
        let mut model = cm("|");
        model.select(Location::from(7), Location::from(7));
        replace_text(&mut model, "Z");
        assert_eq!(tx(&mut model), "Z|");
    }

    #[test]
    fn replacing_a_selection_with_a_character() {
        let mut model = cm("abc{def}|ghi");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&mut model), "abcZ|ghi");
    }

    #[test]
    fn replacing_a_backwards_selection_with_a_character() {
        let mut model = cm("abc|{def}ghi");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&mut model), "abcZ|ghi");
    }

    #[test]
    fn typing_a_character_after_a_multi_codepoint_character() {
        // Woman Astronaut:
        // Woman+Dark Skin Tone+Zero Width Joiner+Rocket
        let mut model = cm("\u{1F469}\u{1F3FF}\u{200D}\u{1F680}|");
        replace_text(&mut model, "Z");
        assert_eq!(tx(&mut model), "\u{1F469}\u{1F3FF}\u{200D}\u{1F680}Z|");
    }

    #[test]
    fn backspacing_a_character_at_the_end_deletes_it() {
        let mut model = cm("abc|");
        model.backspace();
        assert_eq!(tx(&mut model), "ab|");
    }

    #[test]
    fn backspacing_a_character_at_the_beginning_does_nothing() {
        let mut model = cm("|abc");
        model.backspace();
        assert_eq!(tx(&mut model), "|abc");
    }

    #[test]
    fn backspacing_a_character_in_the_middle_deletes_it() {
        let mut model = cm("ab|c");
        model.backspace();
        assert_eq!(tx(&mut model), "a|c");
    }

    #[test]
    fn backspacing_a_selection_deletes_it() {
        let mut model = cm("a{bc}|");
        model.backspace();
        assert_eq!(tx(&mut model), "a|");
    }

    #[test]
    fn backspacing_a_backwards_selection_deletes_it() {
        let mut model = cm("a|{bc}");
        model.backspace();
        assert_eq!(tx(&mut model), "a|");
    }

    #[test]
    fn deleting_a_character_at_the_end_does_nothing() {
        let mut model = cm("abc|");
        model.delete();
        assert_eq!(tx(&mut model), "abc|");
    }

    #[test]
    fn deleting_a_character_at_the_beginning_deletes_it() {
        let mut model = cm("|abc");
        model.delete();
        assert_eq!(tx(&mut model), "|bc");
    }

    #[test]
    fn deleting_a_character_in_the_middle_deletes_it() {
        let mut model = cm("a|bc");
        model.delete();
        assert_eq!(tx(&mut model), "a|c");
    }

    #[test]
    fn deleting_a_selection_deletes_it() {
        let mut model = cm("a{bc}|");
        model.delete();
        assert_eq!(tx(&mut model), "a|");
    }

    #[test]
    fn deleting_a_backwards_selection_deletes_it() {
        let mut model = cm("a|{bc}");
        model.delete();
        assert_eq!(tx(&mut model), "a|");
    }

    #[test]
    fn selecting_ascii_characters() {
        let mut model = cm("abcdefgh|");
        model.select(Location::from(0), Location::from(1));
        assert_eq!(tx(&mut model), "{a}|bcdefgh");

        model.select(Location::from(1), Location::from(3));
        assert_eq!(tx(&mut model), "a{bc}|defgh");

        model.select(Location::from(4), Location::from(8));
        assert_eq!(tx(&mut model), "abcd{efgh}|");

        model.select(Location::from(4), Location::from(9));
        assert_eq!(tx(&mut model), "abcd{efgh}|");
    }

    #[test]
    fn selecting_single_utf16_code_unit_characters() {
        let mut model = cm("\u{03A9}\u{03A9}\u{03A9}|");

        model.select(Location::from(0), Location::from(1));
        assert_eq!(tx(&mut model), "{\u{03A9}}|\u{03A9}\u{03A9}");

        model.select(Location::from(0), Location::from(3));
        assert_eq!(tx(&mut model), "{\u{03A9}\u{03A9}\u{03A9}}|");

        model.select(Location::from(1), Location::from(2));
        assert_eq!(tx(&mut model), "\u{03A9}{\u{03A9}}|\u{03A9}");
    }

    #[test]
    fn selecting_multiple_utf16_code_unit_characters() {
        let mut model = cm("\u{1F4A9}\u{1F4A9}\u{1F4A9}|");

        model.select(Location::from(0), Location::from(2));
        assert_eq!(tx(&mut model), "{\u{1F4A9}}|\u{1F4A9}\u{1F4A9}");

        model.select(Location::from(0), Location::from(6));
        assert_eq!(tx(&mut model), "{\u{1F4A9}\u{1F4A9}\u{1F4A9}}|");

        model.select(Location::from(2), Location::from(4));
        assert_eq!(tx(&mut model), "\u{1F4A9}{\u{1F4A9}}|\u{1F4A9}");
    }

    #[test]
    fn selecting_complex_characters() {
        let mut model =
            cm("aaa\u{03A9}bbb\u{1F469}\u{1F3FF}\u{200D}\u{1F680}ccc|");

        model.select(Location::from(0), Location::from(3));
        assert_eq!(
            tx(&mut model),
            "{aaa}|\u{03A9}bbb\u{1F469}\u{1F3FF}\u{200D}\u{1F680}ccc"
        );

        model.select(Location::from(0), Location::from(4));
        assert_eq!(
            tx(&mut model),
            "{aaa\u{03A9}}|bbb\u{1F469}\u{1F3FF}\u{200D}\u{1F680}ccc"
        );

        model.select(Location::from(7), Location::from(14));
        assert_eq!(
            tx(&mut model),
            "aaa\u{03A9}bbb{\u{1F469}\u{1F3FF}\u{200D}\u{1F680}}|ccc"
        );

        model.select(Location::from(7), Location::from(15));
        assert_eq!(
            tx(&mut model),
            "aaa\u{03A9}bbb{\u{1F469}\u{1F3FF}\u{200D}\u{1F680}c}|cc"
        );
    }

    #[test]
    fn bolding_ascii_adds_strong_tags() {
        let mut model = cm("aa{bb}|cc");
        model.bold();
        // TODO: because it's not an AST
        assert_eq!(tx(&mut model), "aa{<s}|trong>bb</strong>cc");

        let mut model = cm("aa|{bb}cc");
        model.bold();
        assert_eq!(tx(&mut model), "aa|{<s}trong>bb</strong>cc");
    }

    // Test utils

    fn replace_text(model: &mut ComposerModel<u16>, new_text: &str) {
        model.replace_text(&new_text.encode_utf16().collect::<Vec<u16>>());
    }

    trait Roundtrips<T> {
        fn roundtrips(&self);
    }

    impl<'s, T> Roundtrips<T> for Spec<'s, T>
    where
        T: AsRef<str>,
    {
        fn roundtrips(&self) {
            let subject = self.subject.as_ref();
            let output = tx(&mut cm(subject));
            if output != subject {
                AssertionFailure::from_spec(self)
                    .with_expected(String::from(subject))
                    .with_actual(output)
                    .fail();
            }
        }
    }

    /**
     * Create a ComposerModel from a text representation.
     */
    fn cm(text: &str) -> ComposerModel<u16> {
        let text: Vec<u16> = text.encode_utf16().collect();

        fn find(haystack: &[u16], needle: &str) -> Option<usize> {
            let needle = needle.encode_utf16().collect::<Vec<u16>>()[0];
            for (i, &ch) in haystack.iter().enumerate() {
                if ch == needle {
                    return Some(i);
                }
            }
            None
        }

        let curs = find(&text, "|").expect(&format!(
            "ComposerModel text did not contain a '|' symbol: '{}'",
            String::from_utf16(&text)
                .expect("ComposerModel text was not UTF-16"),
        ));

        let s = find(&text, "{");
        let e = find(&text, "}");

        if let (Some(s), Some(e)) = (s, e) {
            if curs == e + 1 {
                let mut html = text[..s].to_vec();
                html.extend_from_slice(&text[s + 1..e]);
                html.extend_from_slice(&text[curs + 1..]);
                // Cursor after end: foo{bar}|baz
                // The { made an extra codeunit - move the end back 1
                ComposerModel::from(html, s, e - 1)
            } else if curs == s - 1 {
                // Cursor before beginning: foo|{bar}baz
                // The |{ made an extra 2 codeunits - move the end back 2
                let mut html = text[..curs].to_vec();
                html.extend_from_slice(&text[s + 1..e]);
                html.extend_from_slice(&text[e + 1..]);
                ComposerModel::from(html, e - 2, curs)
            } else {
                panic!(
                    "The cursor ('|') must always be directly before or after \
                    the selection ('{{..}}')! \
                    E.g.: 'foo|{{bar}}baz' or 'foo{{bar}}|baz'."
                )
            }
        } else {
            let mut html = text[..curs].to_vec();
            html.extend_from_slice(&text[curs + 1..]);
            ComposerModel::from(html, curs, curs)
        }
    }

    /**
     * Convert a ComposerModel to a text representation.
     */
    fn tx(model: &mut ComposerModel<u16>) -> String {
        let mut ret;
        let html = model.html();
        dbg!(&html);
        if model.start == model.end {
            ret = String::from_utf16(&html[..model.start.into()]).unwrap();
            ret.push('|');
            ret += &String::from_utf16(&html[model.start.into()..]).unwrap();
        } else {
            let (s, e) = model.safe_selection();

            ret = String::from_utf16(&html[..s]).unwrap();
            if model.start < model.end {
                ret.push('{');
            } else {
                ret += "|{";
            }
            ret += &String::from_utf16(&html[s..e]).unwrap();
            if model.start < model.end {
                ret += "}|";
            } else {
                ret.push('}');
            }
            ret += &String::from_utf16(&html[e..]).unwrap()
        }
        ret
    }

    #[test]
    fn cm_creates_correct_component_model() {
        assert_eq!(cm("|").start, 0);
        assert_eq!(cm("|").end, 0);
        assert_eq!(cm("|").html(), &[]);

        assert_eq!(cm("a|").start, 1);
        assert_eq!(cm("a|").end, 1);
        assert_eq!(cm("a|").html(), "a".encode_utf16().collect::<Vec<_>>());

        assert_eq!(cm("a|b").start, 1);
        assert_eq!(cm("a|b").end, 1);
        assert_eq!(cm("a|b").html(), "ab".encode_utf16().collect::<Vec<_>>());

        assert_eq!(cm("|ab").start, 0);
        assert_eq!(cm("|ab").end, 0);
        assert_eq!(cm("|ab").html(), "ab".encode_utf16().collect::<Vec<_>>());

        assert_eq!(cm("foo|").start, 3);
        assert_eq!(cm("foo|").end, 3);
        assert_eq!(cm("foo|").html(), "foo".encode_utf16().collect::<Vec<_>>());

        let mut t1 = cm("foo|\u{1F4A9}bar");
        assert_eq!(t1.start, 3);
        assert_eq!(t1.end, 3);
        assert_eq!(
            t1.html(),
            "foo\u{1F4A9}bar".encode_utf16().collect::<Vec<_>>()
        );

        let mut t2 = cm("foo\u{1F4A9}|bar");
        assert_eq!(t2.start, 5);
        assert_eq!(t2.end, 5);
        assert_eq!(
            t2.html(),
            "foo\u{1F4A9}bar".encode_utf16().collect::<Vec<_>>()
        );

        assert_eq!(cm("foo|\u{1F4A9}").start, 3);
        assert_eq!(cm("foo|\u{1F4A9}").end, 3);
        assert_eq!(
            cm("foo|\u{1F4A9}").html(),
            "foo\u{1F4A9}".encode_utf16().collect::<Vec<_>>()
        );

        assert_eq!(cm("foo\u{1F4A9}|").start, 5);
        assert_eq!(cm("foo\u{1F4A9}|").end, 5);
        assert_eq!(
            cm("foo\u{1F4A9}|").html(),
            "foo\u{1F4A9}".encode_utf16().collect::<Vec<_>>()
        );

        assert_eq!(cm("|\u{1F4A9}bar").start, 0);
        assert_eq!(cm("|\u{1F4A9}bar").end, 0);
        assert_eq!(
            cm("|\u{1F4A9}bar").html(),
            "\u{1F4A9}bar".encode_utf16().collect::<Vec<_>>()
        );

        assert_eq!(cm("\u{1F4A9}|bar").start, 2);
        assert_eq!(cm("\u{1F4A9}|bar").end, 2);
        assert_eq!(
            cm("\u{1F4A9}|bar").html(),
            "\u{1F4A9}bar".encode_utf16().collect::<Vec<_>>()
        );

        assert_eq!(cm("{a}|").start, 0);
        assert_eq!(cm("{a}|").end, 1);
        assert_eq!(cm("{a}|").html(), "a".encode_utf16().collect::<Vec<_>>());

        assert_eq!(cm("|{a}").start, 1);
        assert_eq!(cm("|{a}").end, 0);
        assert_eq!(cm("|{a}").html(), "a".encode_utf16().collect::<Vec<_>>());

        assert_eq!(cm("abc{def}|ghi").start, 3);
        assert_eq!(cm("abc{def}|ghi").end, 6);
        assert_eq!(
            cm("abc{def}|ghi").html(),
            "abcdefghi".encode_utf16().collect::<Vec<_>>()
        );

        assert_eq!(cm("abc|{def}ghi").start, 6);
        assert_eq!(cm("abc|{def}ghi").end, 3);
        assert_eq!(
            cm("abc|{def}ghi").html(),
            "abcdefghi".encode_utf16().collect::<Vec<_>>()
        );

        let mut t3 = cm("\u{1F4A9}{def}|ghi");
        assert_eq!(t3.start, 2);
        assert_eq!(t3.end, 5);
        assert_eq!(
            t3.html(),
            "\u{1F4A9}defghi".encode_utf16().collect::<Vec<_>>()
        );

        let mut t4 = cm("\u{1F4A9}|{def}ghi");
        assert_eq!(t4.start, 5);
        assert_eq!(t4.end, 2);
        assert_eq!(
            t4.html(),
            "\u{1F4A9}defghi".encode_utf16().collect::<Vec<_>>()
        );

        let mut t5 = cm("abc{d\u{1F4A9}f}|ghi");
        assert_eq!(t5.start, 3);
        assert_eq!(t5.end, 7);
        assert_eq!(
            t5.html(),
            "abcd\u{1F4A9}fghi".encode_utf16().collect::<Vec<_>>()
        );

        let mut t6 = cm("abc|{d\u{1F4A9}f}ghi");
        assert_eq!(t6.start, 7);
        assert_eq!(t6.end, 3);
        assert_eq!(
            t6.html(),
            "abcd\u{1F4A9}fghi".encode_utf16().collect::<Vec<_>>()
        );

        let mut t7 = cm("abc{def}|\u{1F4A9}ghi");
        assert_eq!(t7.start, 3);
        assert_eq!(t7.end, 6);
        assert_eq!(
            t7.html(),
            "abcdef\u{1F4A9}ghi".encode_utf16().collect::<Vec<_>>()
        );

        let mut t8 = cm("abc|{def}\u{1F4A9}ghi");
        assert_eq!(t8.start, 6);
        assert_eq!(t8.end, 3);
        assert_eq!(
            t8.html(),
            "abcdef\u{1F4A9}ghi".encode_utf16().collect::<Vec<_>>()
        );
    }

    #[test]
    fn cm_and_tx_roundtrip() {
        assert_that!("|").roundtrips();
        assert_that!("a|").roundtrips();
        assert_that!("a|b").roundtrips();
        assert_that!("|ab").roundtrips();
        assert_that!("foo|\u{1F4A9}bar").roundtrips();
        assert_that!("foo\u{1F4A9}|bar").roundtrips();
        assert_that!("foo|\u{1F4A9}").roundtrips();
        assert_that!("foo\u{1F4A9}|").roundtrips();
        assert_that!("|\u{1F4A9}bar").roundtrips();
        assert_that!("\u{1F4A9}|bar").roundtrips();
        assert_that!("{a}|").roundtrips();
        assert_that!("|{a}").roundtrips();
        assert_that!("abc{def}|ghi").roundtrips();
        assert_that!("abc|{def}ghi").roundtrips();
        assert_that!("\u{1F4A9}{def}|ghi").roundtrips();
        assert_that!("\u{1F4A9}|{def}ghi").roundtrips();
        assert_that!("abc{d\u{1F4A9}f}|ghi").roundtrips();
        assert_that!("abc|{d\u{1F4A9}f}ghi").roundtrips();
        assert_that!("abc{def}|\u{1F4A9}ghi").roundtrips();
        assert_that!("abc|{def}\u{1F4A9}ghi").roundtrips();
    }
}

// Copyright 2014 The html5ever Project Developers.
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

//! A simple DOM where every node is owned by its parent.
//!
//! Since ownership is more complicated during parsing, we actually
//! build a different type and then transmute to the public `Node`.
//! This is believed to be memory safe, but if you want to be extra
//! careful you can use `RcDom` instead.
//!
//! **Warning: Unstable.** This module uses unsafe code, has not
//! been thoroughly audited, and the performance gains vs. RcDom
//! have not been demonstrated.

use html5ever::serialize::TraversalScope;
use html5ever::tendril::StrTendril;
use html5ever::tree_builder;
use html5ever::tree_builder::{
    AppendNode, AppendText, NodeOrText, QuirksMode, TreeSink,
};
use html5ever::Attribute;
use html5ever::ExpandedName;
use html5ever::QualName;
use mac::{addrs_of, unwrap_or_return};

use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::default::Default;
use std::fmt::Debug;
use std::mem::{self, transmute};
use std::ops::{Deref, DerefMut};
use std::ptr;

pub use self::NodeEnum::{Comment, Doctype, Document, Element, Text};

#[derive(Debug)]
pub struct OwnedAttribute {
    name: QualName,
    value: String,
}

impl From<&Attribute> for OwnedAttribute {
    fn from(attr: &Attribute) -> Self {
        Self {
            name: attr.name.clone(),
            value: attr.value.to_string(),
        }
    }
}

/// The different kinds of nodes in the DOM.
#[derive(Debug)]
pub enum NodeEnum {
    /// The `Document` itself.
    Document,

    /// A `DOCTYPE` with name, public id, and system id.
    Doctype(String, String, String),

    /// A text node.
    Text(String),

    /// A comment.
    Comment(String),

    /// An element with attributes.
    Element(QualName, Vec<OwnedAttribute>),
}
/// The internal type we use for nodes during parsing.
pub struct SquishyNode {
    node: NodeEnum,
    parent: Handle,
    children: Vec<Handle>,
}

impl SquishyNode {
    fn new(node: NodeEnum) -> SquishyNode {
        SquishyNode {
            node,
            parent: Handle::null(),
            children: vec![],
        }
    }
}

pub struct Handle {
    ptr: *const UnsafeCell<SquishyNode>,
}

impl Handle {
    fn new(ptr: *const UnsafeCell<SquishyNode>) -> Handle {
        Handle { ptr }
    }

    fn null() -> Handle {
        Handle::new(ptr::null())
    }

    fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    fn deref_mut_custom<'a>(&'a self) -> &'a mut SquishyNode {
        unsafe { transmute::<_, &'a mut SquishyNode>((*self.ptr).get()) }
    }
}

impl PartialEq for Handle {
    fn eq(&self, other: &Handle) -> bool {
        self.ptr == other.ptr
    }
}

impl Eq for Handle {}

impl Clone for Handle {
    fn clone(&self) -> Handle {
        Handle::new(self.ptr)
    }
}

impl Copy for Handle {}

// The safety of `Deref` and `DerefMut` depends on the invariant that `Handle`s
// can't escape the `Sink`, because nodes are deallocated by consuming the
// `Sink`.

impl DerefMut for Handle {
    fn deref_mut<'a>(&'a mut self) -> &'a mut SquishyNode {
        unsafe { transmute::<_, &'a mut SquishyNode>((*self.ptr).get()) }
    }
}

impl Deref for Handle {
    type Target = SquishyNode;
    fn deref<'a>(&'a self) -> &'a SquishyNode {
        unsafe { transmute::<_, &'a SquishyNode>((*self.ptr).get()) }
    }
}

fn append(mut new_parent: Handle, mut child: Handle) {
    new_parent.children.push(child);
    let parent = &mut child.parent;
    assert!(parent.is_null());
    *parent = new_parent
}

fn get_parent_and_index(child: Handle) -> Option<(Handle, usize)> {
    if child.parent.is_null() {
        return None;
    }

    let to_find = child;
    match child
        .parent
        .children
        .iter()
        .enumerate()
        .find(|&(_, n)| *n == to_find)
    {
        Some((i, _)) => Some((child.parent, i)),
        None => panic!("have parent but couldn't find in parent's children!"),
    }
}

fn append_to_existing_text(mut prev: Handle, text: &str) -> bool {
    match prev.deref_mut().node {
        Text(ref mut existing) => {
            *existing += text;
            true
        }
        _ => false,
    }
}

pub struct Sink {
    nodes: Vec<Box<UnsafeCell<SquishyNode>>>,
    document: Handle,
    errors: Vec<Cow<'static, str>>,
    quirks_mode: QuirksMode,
}

impl Default for Sink {
    fn default() -> Sink {
        let mut sink = Sink {
            nodes: vec![],
            document: Handle::null(),
            errors: vec![],
            quirks_mode: tree_builder::NoQuirks,
        };
        sink.document = sink.new_node(Document);
        sink
    }
}

impl Sink {
    fn new_node(&mut self, node: NodeEnum) -> Handle {
        self.nodes
            .push(Box::new(UnsafeCell::new(SquishyNode::new(node))));
        let ptr: *const UnsafeCell<SquishyNode> = &**self.nodes.last().unwrap();
        Handle::new(ptr)
    }

    // FIXME(rust-lang/rust#18296): This is separate from remove_from_parent so
    // we can call it.
    fn unparent(&mut self, mut target: Handle) {
        let (mut parent, i) =
            unwrap_or_return!(get_parent_and_index(target), ());
        parent.children.remove(i);
        target.parent = Handle::null();
    }
}

impl TreeSink for Sink {
    type Handle = Handle;
    type Output = OwnedDom;

    fn parse_error(&mut self, msg: Cow<'static, str>) {
        self.errors.push(msg);
    }

    fn get_document(&mut self) -> Handle {
        self.document
    }

    fn set_quirks_mode(&mut self, mode: QuirksMode) {
        self.quirks_mode = mode;
    }

    fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        x == y
    }

    fn elem_name<'a>(&self, target: &'a Handle) -> ExpandedName<'a> {
        match target.node {
            Element(ref name, _) => name.expanded(),
            _ => panic!("not an element!"),
        }
    }

    fn create_element(
        &mut self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: tree_builder::ElementFlags,
    ) -> Self::Handle {
        self.new_node(Element(name, attrs.iter().map(|a| a.into()).collect()))
    }

    fn create_comment(&mut self, text: StrTendril) -> Handle {
        self.new_node(Comment(text.to_string()))
    }

    fn append(&mut self, parent: &Handle, child: NodeOrText<Handle>) {
        // Append to an existing Text node if we have one.
        match child {
            AppendText(ref text) => match parent.children.last() {
                Some(h) => {
                    if append_to_existing_text(*h, &text) {
                        return;
                    }
                }
                _ => (),
            },
            _ => (),
        }

        append(
            *parent,
            match child {
                AppendText(text) => self.new_node(Text(text.to_string())),
                AppendNode(node) => node,
            },
        );
    }

    fn append_before_sibling(
        &mut self,
        sibling: &Handle,
        child: NodeOrText<Handle>,
    ) {
        let (mut parent, i) =
            get_parent_and_index(*sibling).expect("No parent found!");

        let mut child = match (child, i) {
            // No previous node.
            (AppendText(text), 0) => self.new_node(Text(text.to_string())),

            // Look for a text node before the insertion point.
            (AppendText(text), i) => {
                let prev = parent.children[i - 1];
                if append_to_existing_text(prev, &text) {
                    return;
                }
                self.new_node(Text(text.to_string()))
            }

            // The tree builder promises we won't have a text node after
            // the insertion point.

            // Any other kind of node.
            (AppendNode(node), _) => node,
        };

        if !child.parent.is_null() {
            self.unparent(child);
        }

        child.parent = parent;
        parent.children.insert(i, child);
    }

    fn append_doctype_to_document(
        &mut self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        append(
            self.document,
            self.new_node(Doctype(
                name.to_string(),
                public_id.to_string(),
                system_id.to_string(),
            )),
        );
    }

    fn add_attrs_if_missing(
        &mut self,
        target: &Handle,
        mut attrs: Vec<Attribute>,
    ) {
        let existing = match target.deref_mut_custom().node {
            Element(_, ref mut attrs) => attrs,
            _ => return,
        };

        // FIXME: quadratic time
        attrs.retain(|attr| !existing.iter().any(|e| e.name == attr.name));
        existing.extend::<Vec<OwnedAttribute>>(
            attrs.iter().map(|a| a.into()).collect(),
        );
    }

    fn remove_from_parent(&mut self, target: &Handle) {
        self.unparent(*target);
    }

    fn reparent_children(&mut self, node: &Handle, new_parent: &Handle) {
        new_parent
            .deref_mut_custom()
            .children
            .append(&mut node.deref_mut_custom().children);
    }

    fn mark_script_already_started(&mut self, _node: &Handle) {}

    fn finish(self) -> Self::Output {
        fn walk(live: &mut HashSet<usize>, node: Handle) {
            live.insert(node.ptr as usize);
            for &child in node.deref().children.iter() {
                walk(live, child);
            }
        }

        // Collect addresses of all the nodes that made it into the final tree.
        let mut live = HashSet::new();
        walk(&mut live, self.document);

        // Forget about the nodes in the final tree; they will be owned by
        // their parent.  In the process of iterating we drop all nodes that
        // aren't in the tree.
        for node in self.nodes.into_iter() {
            let ptr: *const UnsafeCell<SquishyNode> = &*node;
            if live.contains(&(ptr as usize)) {
                mem::forget(node);
            }
        }

        let old_addrs = addrs_of!(self.document => node, parent, children);

        // Transmute the root to a Node, finalizing the transfer of ownership.
        let document = unsafe {
            mem::transmute::<*const UnsafeCell<SquishyNode>, Box<Node>>(
                self.document.ptr,
            )
        };

        // FIXME: do this assertion statically
        let new_addrs =
            addrs_of!(document => node, _parent_not_accessible, children);
        assert_eq!(old_addrs, new_addrs);

        OwnedDom {
            document,
            errors: self.errors,
            quirks_mode: self.quirks_mode,
        }
    }

    fn create_pi(
        &mut self,
        target: StrTendril,
        data: StrTendril,
    ) -> Self::Handle {
        todo!()
    }

    fn append_based_on_parent_node(
        &mut self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        todo!()
    }

    fn get_template_contents(&mut self, target: &Self::Handle) -> Self::Handle {
        todo!()
    }
}

pub struct Node {
    pub node: NodeEnum,
    _parent_not_accessible: usize,
    pub children: Vec<Box<Node>>,
}

pub struct OwnedDom {
    pub document: Box<Node>,
    pub errors: Vec<Cow<'static, str>>,
    pub quirks_mode: QuirksMode,
}

impl std::fmt::Display for OwnedDom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn excluded_tag(local: &str) -> bool {
            match local {
                "html" => true,
                "head" => true,
                "body" => true,
                _ => false,
            }
        }

        fn imp(
            f: &mut std::fmt::Formatter<'_>,
            parent: &Box<Node>,
        ) -> std::fmt::Result {
            match &parent.node {
                Text(text) => {
                    f.write_str(&text)?;
                }
                Element(qualname, _attrs) => {
                    if !excluded_tag(&qualname.local) {
                        f.write_fmt(format_args!("<{}>", qualname.local))?;
                        // TODO: attrs
                    }
                }
                _ => {}
            }
            for node in &parent.children {
                imp(f, &node)?;
            }
            match &parent.node {
                Element(qualname, _attrs) => {
                    if !excluded_tag(&qualname.local) {
                        f.write_fmt(format_args!("</{}>", qualname.local))?;
                        // TODO: attrs
                    }
                }
                _ => {}
            };
            Ok(())
        }

        imp(f, &self.document)

        /*
        let traversal_scope = TraversalScope::IncludeNode;
        match (traversal_scope, &self.node) {
            (_, &Element(ref name, ref attrs)) => {
                if traversal_scope == IncludeNode {
                    serializer.start_elem(
                        name.clone(),
                        attrs.iter().map(|at| (&at.name, &at.value[..])),
                    )?;
                }

                for child in self.children.iter() {
                    child.serialize(serializer, IncludeNode)?;
                }

                if traversal_scope == IncludeNode {
                    serializer.end_elem(name.clone())?;
                }
                Ok(())
            }

            (TraversalScope::ChildrenOnly(), &Document) => {
                for child in self.children.iter() {
                    child.serialize(serializer, IncludeNode)?;
                }
                Ok(())
            }

            (TraversalScope::ChildrenOnly(), _) => Ok(()),

            (IncludeNode, &Doctype(ref name, _, _)) => {
                serializer.write_doctype(&name)
            }
            (IncludeNode, &Text(ref text)) => serializer.write_text(&text),
            (IncludeNode, &Comment(ref text)) => {
                serializer.write_comment(&text)
            }

            (IncludeNode, &Document) => {
                panic!("Can't serialize Document node itself")
            }
        }*/
    }
}

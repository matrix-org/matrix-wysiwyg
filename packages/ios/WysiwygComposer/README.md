# WysiwygComposer

This package contains most of the source code powering our WYSIWYG editor.

It is composed of: 
* Static libraries generated from Rust, packaged in a XCFramework.
* Swift bindings to interact with these libraries.
* Components built on top of these bindings.
* Unit tests validating this.
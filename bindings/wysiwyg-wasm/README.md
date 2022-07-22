# `wysiwyg-wasm`

WASM/JavaScript bindings for wysiwyg-rust.

## Building

* [Install Rust](https://www.rust-lang.org/tools/install)
* [Install NodeJS and NPM](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm)
* [Install wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
* Run (in the top level directory):

```bash
make web
```

This effectively does:

```sh
cd bindings/wysiwyg-wasm
npm install
npm run build
```

which generates:

```
pkg/matrix_sdk_wysiwyg_bg.wasm
pkg/matrix_sdk_wysiwyg_bg.wasm.d.ts
pkg/matrix_sdk_wysiwyg.d.ts
pkg/matrix_sdk_wysiwyg.js
... plus other files
```

and copies those files into examples/example-web/generated/

To run the demo:

```bash
cd examples/example-web/
python -m http.server
```

And navigate to http://0.0.0.0:8000/ in your web browser.

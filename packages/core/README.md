# tagged-md

Transform tagged Markdown string to HTML!

## Note

This package actually doesn't contain any functionality, but is a placeholder module that to be replaced by the build time transformer plugins.

Currently, the following plugins are available:

- [swc-plugin-tagged-md](https://npmjs.com/package/swc-plugin-tagged-md) - for [swc](https://npmjs.com/package/swc)

Make sure to install one of them to use this package.

## Usage

```tsx
import { md } from "tagged-md"

const content = md`
# Hello, world!

This content will be transformed into HTML **during build time.**
`
```

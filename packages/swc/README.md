# swc-plugin-tagged-md

SWC plugin for transforming tagged Markdown string to HTML.

## Usage

Install `swc` and this plugin, then configure `swc` to use this plugin.

```tsx
const md = String.raw

const content = md`
# Hello, world!

This content will be transformed into HTML **during build time.**
`
```

# swc-plugin-tagged-md

SWC plugin for transforming tagged Markdown string to HTML.

## Usage

Install `swc`, this plugin, and `tagged-md`, then configure `swc` to use this plugin.

```tsx
import { md } from "tagged-md"

const content = md`
# Hello, world!

This content will be transformed into HTML **during build time.**
`
```

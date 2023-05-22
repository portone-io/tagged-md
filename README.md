# tagged-md

Transform tagged Markdown string to HTML!

[Check out the examples!](examples)

## Motivation

Have you ever written HTML strings in your JavaScript code? It's usually needed for adding formatted strings.
However, writing raw HTML tags in string literal is inconvenient and typo-prone. It would be much better if we could use Markdown directly in those strings.
But, if you just write strings in Markdown and transform them into HTML at runtime,
you should include a Markdown parser in your application bundle!
That's not good for both loading and computing performance. Wouldn't it be better to transform them into HTML at build time?
`tagged-md` is made for exactly that purpose.

## Installation

`tagged-md` requires installing both the main package, and the plugin that integrates with transpilers.

Currently, the following plugins are available:

- [swc-plugin-tagged-md](https://npmjs.com/package/swc-plugin-tagged-md) - for [swc](https://npmjs.com/package/swc)

If you need plugins for other transpilers, please create an issue for it!

Install the main package, and the plugin of your choice.

```sh
npm install -D tagged-md swc-plugin-tagged-md    # using npm and swc
yarn add -D tagged-md swc-plugin-tagged-md       # using yarn and swc
pnpm install -D tagged-md swc-plugin-tagged-md   # using pnpm and swc
```

## Usage

First, make sure to configure the plugin you've installed by following each plugin's documentation.

You may write Markdown string literals like this:

```tsx
import { md } from "tagged-md";

const content = md`
# Hello, world!

This content will be transformed into HTML **during build time.**
`
```

You can also apply Markdown configurations (like using GitHub-flavored Markdown) per each string literal.

```tsx
import { md } from "tagged-md";

const issueBody = md({ gfm: true })`
# Tasks

- [x] Install tagged-md
- [ ] Write contents
`
```

Then the plugin will transform them into HTML while transpiling!

--------

Packages under *portone-io/tagged-md* are primarily distributed under the terms of
both the [Apache License (Version 2.0)] and the [MIT license]. See [COPYRIGHT]
for details.

[MIT license]: LICENSE-MIT
[Apache License (Version 2.0)]: LICENSE-APACHE
[COPYRIGHT]: COPYRIGHT

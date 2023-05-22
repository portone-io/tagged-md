# swc-plugin-tagged-md

SWC plugin for transforming tagged Markdown string to HTML.

## Configuration

Install this plugin and `tagged-md`.

```sh
npm install -D tagged-md swc-plugin-tagged-md   # for npm
yarn add -D tagged-md swc-plugin-tagged-md      # for yarn
pnpm install -D tagged-md swc-plugin-tagged-md  # for pnpm
```

Then configure `swc` to use this plugin using `.swcrc`. The empty object in the array is for configuring the plugin.

```json
{
  "jsc": {
    "experimental": {
      "plugins": [["swc-plugin-tagged-md", {}]]
    }
  }
}
```

Note that loading plugins like above will only work with `npm`.
If you use other package managers, you should resolve the actual path of the package using `require.resolve("swc-plugin-tagged-md")`.

Refer to the [main documentation](https://github.com/portone-io/tagged-md/blob/main/README.md) for actual usage!

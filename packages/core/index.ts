export const md: (arr: TemplateStringsArray) => string = () => {
  throw new Error(
    `
Error: the \`md\` tagged template literal should've been replaced with a normal template literal by one of the provided build time transformer plugins.
Please make sure that you have configured your environment correctly to apply one of those.
Currently, the following plugins are available:

- swc-plugin-tagged-md

If you have configured everything correctly and still see this error, please file an issue at: https://github.com/portone-io/tagged-md/issues/new
`
  );
};

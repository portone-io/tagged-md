use std::borrow::Cow;

use derive_builder::Builder;
use serde::Deserialize;
use swc_core::common::*;
use swc_core::ecma::ast::{
    Callee, ExprOrSpread, Ident, ImportDecl, ImportSpecifier, Lit, ModuleExportName, Prop,
    PropName, PropOrSpread, Tpl, TplElement,
};
use swc_core::ecma::{
    ast::{Expr, Program},
    visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
};
use swc_core::plugin::errors::HANDLER;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

struct TplElementInfo {
    span: Span,
    tail: bool,
}

#[derive(Deserialize, Clone, Builder)]
pub struct PluginConfig {
    #[serde(alias = "interpolationPlaceholder")]
    #[serde(default = "default_interpolation_placeholder")]
    #[builder(default = "default_interpolation_placeholder()")]
    interpolation_placeholder: String,
    #[serde(default)]
    #[builder(default)]
    gfm: bool,
}

impl PluginConfig {
    fn try_from_ast(ast: &Expr) -> Result<Self, &str> {
        match ast {
            Expr::Object(object) => {
                let mut config_builder = PluginConfigBuilder::default();

                for item in object.props.iter() {
                    match item {
                        PropOrSpread::Prop(prop) => match &**prop {
                            Prop::KeyValue(kv) => {
                                let key = match &kv.key {
                                    PropName::Ident(ident) => ident.sym.to_string(),
                                    PropName::Str(str) => str.value.to_string(),
                                    _ => return Err("Only static string keys are supported in the config literal."),
                                };
                                match key.as_str() {
                                    "interpolationPlaceholder" => {
                                        if let Expr::Lit(lit) = &*kv.value {
                                            if let Lit::Str(str) = &lit {
                                                config_builder.interpolation_placeholder(
                                                    str.value.clone().to_string(),
                                                );
                                            } else {
                                                return Err("Expected a string literal for the `interpolationPlaceholder` config.");
                                            }
                                        } else {
                                            return Err("Expected a string literal for the `interpolationPlaceholder` config.");
                                        }
                                    }
                                    "gfm" => {
                                        if let Expr::Lit(lit) = &*kv.value {
                                            if let Lit::Bool(boolean) = &lit {
                                                config_builder.gfm(boolean.value);
                                            } else {
                                                return Err("Expected a boolean literal for the `gfm` config.");
                                            }
                                        } else {
                                            return Err(
                                                "Expected a boolean literal for the `gfm` config.",
                                            );
                                        }
                                    }
                                    _ => return Err("Unknown key in the config literal."),
                                }
                            }
                            _ => return Err(
                                "Only key-value properties are supported in the config literal.",
                            ),
                        },
                        PropOrSpread::Spread(_) => {
                            return Err("Spreads in the config literal are not supported.")
                        }
                    }
                }

                Ok(config_builder.build().unwrap())
            }
            _ => Err("Expected an object literal."),
        }
    }
}

pub fn default_interpolation_placeholder() -> String {
    r#"!TAGGED_MD_INTERPOLATION_PLACEHOLDER!"#.to_string()
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            interpolation_placeholder: default_interpolation_placeholder(),
            gfm: false,
        }
    }
}

pub struct TransformVisitor {
    config: PluginConfig,
    tag_idents: Vec<Ident>,
}

impl TransformVisitor {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            tag_idents: vec![],
        }
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_import_decl(&mut self, decl: &mut ImportDecl) {
        decl.visit_mut_children_with(self);

        if !decl.type_only && decl.src.value.eq("tagged-md") {
            for specifier in &decl.specifiers {
                if let ImportSpecifier::Named(named) = specifier {
                    if named
                        .imported
                        .as_ref()
                        .map(|name| match name {
                            ModuleExportName::Ident(ident) => &ident.sym,
                            ModuleExportName::Str(str) => &str.value,
                        })
                        .unwrap_or(&named.local.sym)
                        .eq("md")
                    {
                        self.tag_idents.push(named.local.clone());
                    }
                }
            }
        }
    }
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);

        if let Expr::TaggedTpl(tpl) = expr {
            let (should_transform, config) = match tpl.tag.as_mut() {
                Expr::Ident(ident) => (
                    self.tag_idents
                        .iter()
                        .any(|tag_ident| tag_ident.eq_ignore_span(&ident)),
                    Cow::Borrowed(&self.config),
                ),
                Expr::Call(call) => {
                    let should_transform = match &call.callee {
                        Callee::Expr(expr) => match &**expr {
                            Expr::Ident(ident) => self
                                .tag_idents
                                .iter()
                                .any(|tag_ident| tag_ident.eq_ignore_span(&ident)),
                            _ => false,
                        },
                        _ => false,
                    };
                    if call.args.len() != 1 {
                        HANDLER.with(|handler| {
                            handler
                                .struct_span_err(
                                    call.span,
                                    "Expected exactly one argument to `md` function.",
                                )
                                .emit();
                        });
                        return;
                    }

                    let config = match call.args.first().unwrap() {
                        ExprOrSpread { spread: None, expr } => {
                            match PluginConfig::try_from_ast(&*expr) {
                                Ok(config) => config,
                                Err(err) => {
                                    HANDLER.with(|handler| {
                                        handler.struct_span_err(call.span, err).emit();
                                    });
                                    return;
                                }
                            }
                        }
                        _ => {
                            HANDLER.with(|handler| {
                                handler
                                    .struct_span_err(
                                        call.span,
                                        "Spread arguments are not supported in `md` function.",
                                    )
                                    .emit();
                            });
                            return;
                        },
                    };

                    (should_transform, Cow::Owned(config))
                }
                _ => return,
            };

            if !should_transform {
                return;
            }

            let (element_strings, infos) = tpl
                .tpl
                .quasis
                .iter()
                .map(|q| {
                    (
                        q.cooked.clone().unwrap_or_else(|| q.raw.clone()),
                        TplElementInfo {
                            span: q.span.clone(),
                            tail: q.tail,
                        },
                    )
                })
                .fold(
                    (vec![], vec![]),
                    |(mut str_vec, mut info_vec), (str, info)| {
                        str_vec.push(str);
                        info_vec.push(info);
                        (str_vec, info_vec)
                    },
                );
            let interpolation_replaced = element_strings.join(&config.interpolation_placeholder);
            let lines = interpolation_replaced.lines();
            let mut min_indent = 0;
            let merged = lines
                .map(|line| {
                    if !line.is_empty() {
                        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                        if min_indent == 0 || indent < min_indent {
                            min_indent = indent;
                        }
                        line.chars().skip(min_indent).collect::<String>()
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let transformed = markdown::to_html_with_options(
                &merged,
                &match config.as_ref() {
                    PluginConfig { gfm: true, .. } => markdown::Options::gfm(),
                    PluginConfig { gfm: false, .. } => markdown::Options::default(),
                },
            );
            match transformed {
                Ok(transformed) => {
                    *expr = Tpl {
                        exprs: tpl.tpl.exprs.clone(),
                        span: tpl.span,
                        quasis: transformed
                            .split(&config.interpolation_placeholder)
                            .zip(infos)
                            .map(|(s, info)| TplElement {
                                span: info.span,
                                cooked: Some(s.into()),
                                raw: s.into(),
                                tail: info.tail,
                            })
                            .collect(),
                    }
                    .into();
                }
                Err(error) => {
                    HANDLER.with(|handler| {
                        handler
                            .struct_span_err(
                                tpl.span,
                                &format!("Failed to transform Markdown: {}", error),
                            )
                            .emit();
                    });
                }
            }
        }
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config = metadata
        .get_transform_plugin_config()
        .map(|json| serde_json::from_str::<PluginConfig>(&json).unwrap())
        .unwrap_or_default();
    program.fold_with(&mut as_folder(TransformVisitor::new(config)))
}

#[cfg(test)]
mod tests {
    use swc_core::ecma::{
        transforms::{base::resolver, testing::test},
        visit::as_folder,
    };

    use crate::*;

    test!(
        Default::default(),
        |_| as_folder(TransformVisitor::new(PluginConfig::default())),
        processes_markdown_paragraph,
        r#"
import { md } from "tagged-md";
console.log(md`foo`);
"#,
        r#"
import { md } from "tagged-md";
console.log(`<p>foo</p>`);
"#
    );

    test!(
        Default::default(),
        |_| as_folder(TransformVisitor::new(PluginConfig::default())),
        processes_markdown_bold,
        r#"
import { md } from "tagged-md";
console.log(md`**foo**`);
"#,
        r#"
import { md } from "tagged-md";
console.log(`<p><strong>foo</strong></p>`);
"#
    );

    test!(
        Default::default(),
        |_| as_folder(TransformVisitor::new(PluginConfig::default())),
        processes_escaped_markdown,
        r#"
import { md } from "tagged-md";
console.log(md`**\`foo\`**`);
"#,
        r#"
import { md } from "tagged-md";
console.log(`<p><strong><code>foo</code></strong></p>`);
"#
    );

    test!(
        Default::default(),
        |_| as_folder(TransformVisitor::new(PluginConfig::default())),
        deindents_indented_markdown,
        r#"
import { md } from "tagged-md";
console.log(md`
    # Yay

    **\`foo\`**
`);"#,
        r#"
import { md } from "tagged-md";
console.log(`<h1>Yay</h1>
<p><strong><code>foo</code></strong></p>`);"#
    );

    test!(
        Default::default(),
        |_| as_folder(TransformVisitor::new(PluginConfig::default())),
        processes_expression_interpolation,
        r#"
import { md } from "tagged-md";
console.log(md`**\`${foo}\`**`);
"#,
        r#"
import { md } from "tagged-md";
console.log(`<p><strong><code>${foo}</code></strong></p>`);
"#
    );

    test!(
        Default::default(),
        |_| as_folder(TransformVisitor::new(PluginConfig::default())),
        processes_complex_expression_interpolation,
        r#"
import { md } from "tagged-md";
md`
    The identifier of the PG module and the store ${"I" + "D"} to use.

    Should be written in the following format.

    **\`{PG module identifier}.{Store ID}\`**
`"#,
        r#"
import { md } from "tagged-md";
`<p>The identifier of the PG module and the store ${"I" + "D"} to use.</p>
<p>Should be written in the following format.</p>
<p><strong><code>{PG module identifier}.{Store ID}</code></strong></p>`"#
    );

    test!(
        Default::default(),
        |_| chain!(
            resolver(Mark::new(), Mark::new(), false),
            as_folder(TransformVisitor::new(PluginConfig::default()))
        ),
        only_processes_tag_from_module,
        r#"
import { md } from "tagged-md";
import { md as markdown } from "tagged-md";

const str = md`**foo**`;
const str2 = markdown`**foo**`;
{
    const md = String.raw;
    const str = md`**foo**`;
}
"#,
        r#"
import { md } from "tagged-md";
import { md as markdown } from "tagged-md";

const str = `<p><strong>foo</strong></p>`;
const str2 = `<p><strong>foo</strong></p>`;
{
    const md = String.raw;
    const str = md`**foo**`;
}
"#
    );
}

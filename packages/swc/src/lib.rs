use swc_core::common::*;
use swc_core::ecma::ast::{Ident, ImportDecl, ImportSpecifier, ModuleExportName, Tpl, TplElement};
use swc_core::ecma::{
    ast::{Expr, Program},
    visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

const INTERPOLATION_PLACEHOLDER: &str = r#"!TAGGED_MD_INTERPOLATION_PLACEHOLDER!"#;

struct TplElementInfo {
    span: Span,
    tail: bool,
}

pub struct TransformVisitor {
    tag_idents: Vec<Ident>,
}

impl TransformVisitor {
    pub fn new() -> Self {
        Self { tag_idents: vec![] }
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
            let should_transform = match tpl.tag.as_mut() {
                Expr::Ident(ident) => self
                    .tag_idents
                    .iter()
                    .any(|tag_ident| tag_ident.eq_ignore_span(&ident)),
                _ => false,
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
            let interpolation_replaced = element_strings.join(INTERPOLATION_PLACEHOLDER);
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
            let transformed = markdown::to_html(&merged);
            *expr = Tpl {
                exprs: tpl.tpl.exprs.clone(),
                span: tpl.span,
                quasis: transformed
                    .split(INTERPOLATION_PLACEHOLDER)
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
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut as_folder(TransformVisitor::new()))
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
        |_| as_folder(TransformVisitor::new()),
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
        |_| as_folder(TransformVisitor::new()),
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
        |_| as_folder(TransformVisitor::new()),
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
        |_| as_folder(TransformVisitor::new()),
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
        |_| as_folder(TransformVisitor::new()),
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
        |_| as_folder(TransformVisitor::new()),
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
            as_folder(TransformVisitor::new())
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

use swc_core::common::Span;
use swc_core::ecma::ast::{Tpl, TplElement};
use swc_core::ecma::{
    ast::{Expr, Program},
    transforms::testing::test,
    visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

pub struct TransformVisitor;

const INTERPOLATION_PLACEHOLDER: &str = r#"!TAGGED_MD_INTERPOLATION_PLACEHOLDER!"#;

struct TplElementInfo {
    span: Span,
    tail: bool,
}

impl VisitMut for TransformVisitor {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);

        if let Expr::TaggedTpl(tpl) = expr {
            if let Expr::Ident(ident) = tpl.tag.as_mut() {
                if ident.sym.eq("md") {
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
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut as_folder(TransformVisitor))
}

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_markdown_paragraph,
    r#"console.log(md`foo`);"#,
    r#"console.log(`<p>foo</p>`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_markdown_bold,
    r#"console.log(md`**foo**`);"#,
    r#"console.log(`<p><strong>foo</strong></p>`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_escaped_markdown,
    r#"console.log(md`**\`foo\`**`);"#,
    r#"console.log(`<p><strong><code>foo</code></strong></p>`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    deindents_indented_markdown,
    r#"console.log(md`
        # Yay

        **\`foo\`**
    `);"#,
    r#"console.log(`<h1>Yay</h1>
<p><strong><code>foo</code></strong></p>
`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_expression_interpolation,
    r#"console.log(md`**\`${foo}\`**`);"#,
    r#"console.log(`<p><strong><code>${foo}</code></strong></p>`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_complex_expression_interpolation,
    r#"md`
    The identifier of the PG module and the store ${"I" + "D"} to use.

    Should be written in the following format.

    **\`{PG module identifier}.{Store ID}\`**
  `"#,
    r#"`<p>The identifier of the PG module and the store ${"I" + "D"} to use.</p>
<p>Should be written in the following format.</p>
<p><strong><code>{PG module identifier}.{Store ID}</code></strong></p>
`"#
);

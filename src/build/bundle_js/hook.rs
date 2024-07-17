//! Provides [`Hook`].

/// SWC bundle hook.
pub struct Hook;

impl swc_core::bundler::Hook for Hook {
    fn get_import_meta_props(
        &self,
        span: swc_core::common::Span,
        module_record: &swc_core::bundler::ModuleRecord,
    ) -> Result<Vec<swc_core::ecma::ast::KeyValueProp>, anyhow::Error> {
        use swc_core::ecma::ast::{
            Bool, Expr, Ident, KeyValueProp, Lit, MemberExpr, MemberProp, MetaPropExpr,
            MetaPropKind, PropName, Str,
        };

        let file_name = module_record.file_name.to_string();

        Ok(vec![
            KeyValueProp {
                key: PropName::Ident(Ident::new("url".into(), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    raw: None,
                    value: file_name.into(),
                }))),
            },
            KeyValueProp {
                key: PropName::Ident(Ident::new("main".into(), span)),
                value: Box::new(if module_record.is_entry {
                    Expr::Member(MemberExpr {
                        span,
                        obj: Box::new(Expr::MetaProp(MetaPropExpr {
                            span,
                            kind: MetaPropKind::ImportMeta,
                        })),
                        prop: MemberProp::Ident(Ident::new("main".into(), span)),
                    })
                } else {
                    Expr::Lit(Lit::Bool(Bool { span, value: false }))
                }),
            },
        ])
    }
}

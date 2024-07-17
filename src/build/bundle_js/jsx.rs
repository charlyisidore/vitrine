//! Provides [`jsx_dom_expressions`].

use jsx_dom_expressions::{config::Config, TransformVisitor};
use swc_core::{
    common::comments::Comments,
    ecma::visit::{as_folder, Fold, VisitMut},
};

/// JSX DOM expressions.
pub fn jsx_dom_expressions<C>(comments: C) -> impl Fold + VisitMut
where
    C: Comments,
{
    as_folder(TransformVisitor::new(
        Config {
            module_name: "solid-js/web".to_string(),
            built_ins: vec![
                "For".into(),
                "Show".into(),
                "Switch".into(),
                "Match".into(),
                "Suspense".into(),
                "SuspenseList".into(),
                "Portal".into(),
                "Index".into(),
                "Dynamic".into(),
                "ErrorBoundary".into(),
            ],
            ..Default::default()
        },
        comments,
    ))
}

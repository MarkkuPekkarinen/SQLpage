use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArgumentList, FunctionArguments, Ident,
    ObjectName, SelectItem, SetExpr, Statement, Value,
};

use super::{DelayedFunctionCall, SqlPageFunctionName, extract_sqlpage_function_name};
use crate::webserver::database::sql::parameter_extraction::{
    ParamExtractContext, function_args_to_stmt_params,
};

/// The execution of standalone projected `SQLPage` functions is delayed until after
/// the query has been executed. For instance, `SELECT sqlpage.fetch(x) AS body FROM t`
/// is executed as `SELECT x AS _sqlpage_f0_a0 FROM t`; `sqlpage.fetch` is then
/// called with `_sqlpage_f0_a0` for each returned row.
pub(super) fn extract_delayed_functions_from_query(
    stmt: &mut Statement,
) -> Vec<DelayedFunctionCall> {
    match stmt {
        Statement::Query(q) => {
            let is_limited = q.limit_clause.is_some() || q.fetch.is_some();
            let SetExpr::Select(s) = q.body.as_mut() else {
                return Vec::new();
            };

            let is_constant_single_row = !is_limited && s.from.is_empty() && s.selection.is_none();
            extract_delayed_functions_from_projection(&mut s.projection, is_constant_single_row)
        }
        _ => Vec::new(),
    }
}

fn extract_delayed_functions_from_projection(
    select_items: &mut Vec<SelectItem>,
    is_constant_single_row: bool,
) -> Vec<DelayedFunctionCall> {
    let mut delayed_function_calls = Vec::new();
    let mut rewritten_projection = Vec::with_capacity(select_items.len());
    for item in std::mem::take(select_items) {
        rewrite_select_item(
            item,
            is_constant_single_row,
            &mut rewritten_projection,
            &mut delayed_function_calls,
        );
    }
    *select_items = rewritten_projection;
    delayed_function_calls
}

fn rewrite_select_item(
    item: SelectItem,
    is_constant_single_row: bool,
    rewritten_projection: &mut Vec<SelectItem>,
    delayed_function_calls: &mut Vec<DelayedFunctionCall>,
) {
    match item {
        SelectItem::ExprWithAlias {
            expr: Expr::Function(function),
            alias,
        } => {
            if let Some(func_name) = delayable_sqlpage_function(&function)
                && (!is_constant_single_row || !preserves_null_concat_semantics(&function))
            {
                let (replacement_items, delayed_call) = rewrite_function_projection(
                    function,
                    func_name,
                    alias.value,
                    delayed_function_calls.len(),
                );
                rewritten_projection.extend(replacement_items);
                delayed_function_calls.push(delayed_call);
            } else {
                rewritten_projection.push(SelectItem::ExprWithAlias {
                    expr: Expr::Function(function),
                    alias,
                });
            }
        }
        SelectItem::UnnamedExpr(Expr::Function(function)) => {
            if let Some(func_name) = delayable_sqlpage_function(&function)
                && (!is_constant_single_row || !preserves_null_concat_semantics(&function))
            {
                let target_col_name = function.to_string();
                let (replacement_items, delayed_call) = rewrite_function_projection(
                    function,
                    func_name,
                    target_col_name,
                    delayed_function_calls.len(),
                );
                rewritten_projection.extend(replacement_items);
                delayed_function_calls.push(delayed_call);
            } else {
                rewritten_projection.push(SelectItem::UnnamedExpr(Expr::Function(function)));
            }
        }
        item => rewritten_projection.push(item),
    }
}

fn delayable_sqlpage_function(function: &Function) -> Option<SqlPageFunctionName> {
    let Function {
        name: ObjectName(func_name_parts),
        args:
            FunctionArguments::List(FunctionArgumentList {
                args,
                duplicate_treatment: None,
                ..
            }),
        ..
    } = function
    else {
        return None;
    };
    let func_name = extract_sqlpage_function_name(func_name_parts)?;
    if !args.iter().all(function_arg_is_expr) {
        log::error!("Unsupported argument to {func_name}: {args:?}");
        return None;
    }
    Some(func_name)
}

fn preserves_null_concat_semantics(function: &Function) -> bool {
    let FunctionArguments::List(FunctionArgumentList { args, .. }) = &function.args else {
        return false;
    };
    let mut args = args.clone();
    let has_concat = args.iter().any(function_arg_is_concat);
    has_concat && function_args_to_stmt_params(&mut args, &ParamExtractContext::default()).is_ok()
}

fn function_arg_is_concat(arg: &FunctionArg) -> bool {
    let (FunctionArg::Unnamed(FunctionArgExpr::Expr(expr))
    | FunctionArg::Named {
        arg: FunctionArgExpr::Expr(expr),
        ..
    }) = arg
    else {
        return false;
    };

    match expr {
        Expr::BinaryOp {
            op: sqlparser::ast::BinaryOperator::StringConcat,
            ..
        } => true,
        Expr::Function(Function {
            name: ObjectName(parts),
            ..
        }) => {
            parts.len() == 1
                && parts[0]
                    .as_ident()
                    .is_some_and(|ident| ident.value.eq_ignore_ascii_case("concat"))
        }
        _ => false,
    }
}

fn rewrite_function_projection(
    mut function: Function,
    func_name: SqlPageFunctionName,
    target_col_name: String,
    func_idx: usize,
) -> (Vec<SelectItem>, DelayedFunctionCall) {
    let Function {
        args:
            FunctionArguments::List(FunctionArgumentList {
                args,
                duplicate_treatment: None,
                ..
            }),
        ..
    } = &mut function
    else {
        unreachable!("delayable_sqlpage_function checked the function shape")
    };

    let mut argument_col_names = Vec::with_capacity(args.len());
    let mut replacement_items = Vec::with_capacity(args.len().max(1));
    for (arg_idx, arg) in args.iter_mut().enumerate() {
        let argument_col_name = format!("_sqlpage_f{func_idx}_a{arg_idx}");
        argument_col_names.push(argument_col_name.clone());
        replacement_items.push(SelectItem::ExprWithAlias {
            expr: take_function_arg_expr(arg),
            alias: Ident::with_quote('"', argument_col_name),
        });
    }

    if replacement_items.is_empty() {
        replacement_items.push(SelectItem::ExprWithAlias {
            expr: Expr::value(Value::Null),
            alias: Ident::with_quote('"', target_col_name.clone()),
        });
    }

    (
        replacement_items,
        DelayedFunctionCall {
            function: func_name,
            argument_col_names,
            target_col_name,
        },
    )
}

fn function_arg_is_expr(arg: &FunctionArg) -> bool {
    matches!(
        arg,
        FunctionArg::Unnamed(FunctionArgExpr::Expr(_))
            | FunctionArg::Named {
                arg: FunctionArgExpr::Expr(_),
                ..
            }
    )
}

fn take_function_arg_expr(arg: &mut FunctionArg) -> Expr {
    match arg {
        FunctionArg::Unnamed(FunctionArgExpr::Expr(expr))
        | FunctionArg::Named {
            arg: FunctionArgExpr::Expr(expr),
            ..
        } => std::mem::replace(expr, Expr::value(Value::Null)),
        _ => unreachable!("function_arg_is_expr was checked before taking arguments"),
    }
}

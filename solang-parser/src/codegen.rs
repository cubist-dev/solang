use crate::pt;

pub fn id(name: String) -> pt::Identifier {
    pt::Identifier {
        loc: pt::Loc::Codegen,
        name,
    }
}

pub fn var_expr(name: String) -> pt::Expression {
    pt::Expression::Variable(id(name))
}

pub fn type_expr(ty: pt::Type) -> pt::Expression {
    pt::Expression::Type(pt::Loc::Codegen, ty)
}

pub fn call_expr(name: String, args: Vec<pt::Expression>) -> pt::Expression {
    pt::Expression::FunctionCall(pt::Loc::Codegen, Box::new(var_expr(name)), args)
}

pub fn emit_stmt(name: String, args: Vec<pt::Expression>) -> pt::Statement {
    pt::Statement::Emit(pt::Loc::Codegen, call_expr(name, args))
}

pub fn block_stmt(stmts: Vec<pt::Statement>) -> pt::Statement {
    pt::Statement::Block {
        loc: pt::Loc::Codegen,
        unchecked: true,
        statements: stmts,
    }
}

pub fn event_def(name: String, params: Vec<pt::EventParameter>) -> pt::EventDefinition {
    pt::EventDefinition {
        loc: pt::Loc::Codegen,
        name: id(name),
        fields: params,
        anonymous: false,
    }
}

pub fn function_def(
    name: String,
    params: pt::ParameterList,
    ret: Option<pt::Type>,
    body: pt::Statement,
) -> pt::FunctionDefinition {
    pt::FunctionDefinition {
        loc: pt::Loc::Codegen,
        ty: pt::FunctionTy::Function,
        name: Some(id(name)),
        name_loc: pt::Loc::Codegen,
        params: params,
        attributes: vec![pt::FunctionAttribute::Visibility(pt::Visibility::Public(
            None,
        ))],
        return_not_returns: None,
        returns: annon_parameter_list(ret.map_or_else(Vec::new, |r| vec![r])),
        body: Some(body),
    }
}

pub fn annon_parameter(ty: pt::Type) -> pt::Parameter {
    pt::Parameter {
        loc: pt::Loc::Codegen,
        ty: type_expr(ty),
        storage: None,
        name: None,
    }
}

pub fn event_parameter(name: String, ty: pt::Type) -> pt::EventParameter {
    pt::EventParameter {
        ty: type_expr(ty),
        loc: pt::Loc::Codegen,
        indexed: false,
        name: Some(id(name)),
    }
}

pub fn parameter(name: String, ty: pt::Type) -> pt::Parameter {
    pt::Parameter {
        loc: pt::Loc::Codegen,
        ty: type_expr(ty),
        storage: None,
        name: Some(id(name)),
    }
}

pub fn annon_parameter_list(params: Vec<pt::Type>) -> pt::ParameterList {
    params
        .into_iter()
        .map(|p| (pt::Loc::Codegen, Some(annon_parameter(p))))
        .collect()
}

pub fn parameter_list(params: Vec<(String, pt::Type)>) -> pt::ParameterList {
    params
        .into_iter()
        .map(|p| (pt::Loc::Codegen, Some(parameter(p.0, p.1))))
        .collect()
}

// Parameter conversion

/// Take a list of parameters and convert it to a list of event parameters
pub fn params_to_event_params(params: &pt::ParameterList) -> Vec<pt::EventParameter> {
    params
        .iter()
        .map(|p| {
            assert!(p.1.is_some());
            param_to_event_param(p.1.as_ref().unwrap())
        })
        .collect()
}

/// Take a parameter and convert it to an event parameter
pub fn param_to_event_param(param: &pt::Parameter) -> pt::EventParameter {
    pt::EventParameter {
        ty: param.ty.clone(),
        loc: pt::Loc::Codegen,
        indexed: false,
        name: param.name.clone(),
    }
}

/// Take a list of parameters and convert them to expressions that can be
/// used as a list of arguments
pub fn params_to_args(params: &pt::ParameterList) -> Vec<pt::Expression> {
    params
        .iter()
        .map(|p| {
            assert!(p.1.is_some());
            param_to_arg(p.1.as_ref().unwrap())
        })
        .collect()
}

/// Take a parameter and convert it to an expression that can be used as an argument
pub fn param_to_arg(param: &pt::Parameter) -> pt::Expression {
    assert!(param.name.is_some());
    var_expr(param.name.as_ref().unwrap().name.clone())
}

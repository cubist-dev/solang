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

pub fn call_expr(name: String, args: Vec<String>) -> pt::Expression {
    pt::Expression::FunctionCall(
        pt::Loc::Codegen,
        Box::new(var_expr(name)),
        args.into_iter().map(|x| var_expr(x)).collect(),
    )
}

pub fn emit_stmt(name: String) -> pt::Statement {
    pt::Statement::Emit(pt::Loc::Codegen, var_expr(name))
}

pub fn block_stmt(stmts: Vec<pt::Statement>) -> pt::Statement {
    pt::Statement::Block {
        loc: pt::Loc::Codegen,
        unchecked: true,
        statements: stmts,
    }
}

pub fn event_def(name: String, params: Vec<(String, pt::Type)>) -> pt::EventDefinition {
    pt::EventDefinition {
        loc: pt::Loc::Codegen,
        name: id(name),
        fields: params
            .into_iter()
            .map(|x| event_parameter(x.0, x.1))
            .collect(),
        anonymous: false,
    }
}

pub fn function_def(
    name: String,
    params: Vec<(String, pt::Type)>,
    ret: pt::Type,
    body: pt::Statement,
) -> pt::FunctionDefinition {
    pt::FunctionDefinition {
        loc: pt::Loc::Codegen,
        ty: pt::FunctionTy::Function,
        name: Some(id(name)),
        name_loc: pt::Loc::Codegen,
        params: parameter_list(params),
        attributes: Vec::new(),
        return_not_returns: None,
        returns: annon_parameter_list(vec![ret]),
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

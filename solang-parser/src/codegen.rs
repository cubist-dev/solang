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

pub fn parameter(name: String, ty: pt::Type) -> pt::Parameter {
    pt::Parameter {
        loc: pt::Loc::Codegen,
        ty: type_expr(ty),
        storage: None,
        name: Some(id(name)),
    }
}

pub fn parameter_list(params: Vec<(String, pt::Type)>) -> pt::ParameterList {
    params
        .into_iter()
        .map(|p| (pt::Loc::Codegen, Some(parameter(p.0, p.1))))
        .collect()
}

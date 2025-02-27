// SPDX-License-Identifier: Apache-2.0

use pretty::{Doc, RcDoc};
use std::fmt::{self, Display};
use std::hash::Hash;
use std::hash::Hasher;

#[macro_export]
macro_rules! tern {
    ($condition: expr, $_true: expr, $_false: expr) => {
        if $condition {
            $_true
        } else {
            $_false
        }
    };
}

#[macro_export]
macro_rules! text {
    ($str: expr) => {
        RcDoc::text($str)
    };
}

pub trait Docable {
    fn to_doc(&self) -> RcDoc<()>;
    fn display(&self) -> String {
        let mut s = String::new();
        let doc = self.to_doc();
        doc.render_fmt(70, &mut s).unwrap();
        s
    }
}

pub fn list_to_doc<'a, T: 'a + Docable>(vec: &'a [T]) -> RcDoc<'a> {
    RcDoc::intersperse(
        vec.iter().map(|x| x.to_doc()),
        text!(",").append(RcDoc::space()),
    )
}

pub fn indent_list_to_doc<'a, T: 'a + Docable>(vec: &'a [T]) -> RcDoc<'a> {
    RcDoc::intersperse(
        vec.iter()
            .map(|x| RcDoc::hardline().append(x.to_doc()).nest(4)),
        text!(","),
    )
    .append(RcDoc::hardline())
}

pub fn spaced_list_to_doc<'a, T: 'a + Docable>(vec: &'a [T]) -> RcDoc<'a> {
    RcDoc::intersperse(vec.iter().map(|x| x.to_doc()), RcDoc::space())
}

fn space_if<'a>(cond: bool) -> RcDoc<'a> {
    tern!(cond, RcDoc::space(), RcDoc::nil())
}

pub fn option_to_doc<'a, T: 'a + Docable>(opt: &'a Option<T>) -> RcDoc<'a> {
    opt.as_ref().map(|x| x.to_doc()).unwrap_or_else(RcDoc::nil)
}

pub fn option_space_to_doc<'a, T: 'a + Docable>(opt: &'a Option<T>) -> RcDoc<'a> {
    opt.as_ref()
        .map(|x| x.to_doc().append(RcDoc::space()))
        .unwrap_or_else(RcDoc::nil)
}

pub fn option_box_to_doc<'a, T: 'a + Docable>(opt: &'a Option<Box<T>>) -> RcDoc<'a> {
    opt.as_ref().map(|x| x.to_doc()).unwrap_or_else(RcDoc::nil)
}

pub fn paren_list_to_doc<'a, T: 'a + Docable>(vec: &'a [T]) -> RcDoc<'a> {
    RcDoc::text("(").append(list_to_doc(vec)).append(")")
}

#[derive(Debug, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
/// file no, start offset, end offset (in bytes)
pub enum Loc {
    Builtin,
    CommandLine,
    Implicit,
    Codegen,
    File(usize, usize, usize),
}

impl PartialEq for Loc {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Structs can implement this trait to easily return their loc
pub trait CodeLocation {
    fn loc(&self) -> Loc;
}

/// Structs should implement this trait to return an optional location
pub trait OptionalCodeLocation {
    fn loc(&self) -> Option<Loc>;
}

impl Loc {
    #[must_use]
    pub fn begin_range(&self) -> Self {
        match self {
            Loc::File(file_no, start, _) => Loc::File(*file_no, *start, *start),
            loc => *loc,
        }
    }

    #[must_use]
    pub fn end_range(&self) -> Self {
        match self {
            Loc::File(file_no, _, end) => Loc::File(*file_no, *end, *end),
            loc => *loc,
        }
    }

    pub fn file_no(&self) -> usize {
        match self {
            Loc::File(file_no, _, _) => *file_no,
            _ => unreachable!(),
        }
    }

    /// Return the file_no if the location is in a file
    pub fn try_file_no(&self) -> Option<usize> {
        match self {
            Loc::File(file_no, _, _) => Some(*file_no),
            _ => None,
        }
    }

    pub fn start(&self) -> usize {
        match self {
            Loc::File(_, start, _) => *start,
            _ => unreachable!(),
        }
    }

    pub fn end(&self) -> usize {
        match self {
            Loc::File(_, _, end) => *end,
            _ => unreachable!(),
        }
    }

    pub fn use_end_from(&mut self, other: &Loc) {
        match (self, other) {
            (Loc::File(_, _, end), Loc::File(_, _, other_end)) => {
                *end = *other_end;
            }
            _ => unreachable!(),
        }
    }

    pub fn use_start_from(&mut self, other: &Loc) {
        match (self, other) {
            (Loc::File(_, start, _), Loc::File(_, other_start, _)) => {
                *start = *other_start;
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Identifier {
    pub loc: Loc,
    pub name: String,
}

impl Docable for Identifier {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.to_string())
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IdentifierPath {
    pub loc: Loc,
    pub identifiers: Vec<Identifier>,
}

impl Docable for IdentifierPath {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.to_string())
    }
}

impl Display for IdentifierPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ident) = self.identifiers.get(0) {
            ident.fmt(f)?;
        } else {
            return Ok(());
        }
        for ident in self.identifiers[1..].iter() {
            f.write_str(".")?;
            ident.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Comment {
    Line(Loc, String),
    Block(Loc, String),
    DocLine(Loc, String),
    DocBlock(Loc, String),
}

impl Comment {
    pub fn get_contents(&self) -> &String {
        match self {
            Comment::Line(_, s) => s,
            Comment::Block(_, s) => s,
            Comment::DocLine(_, s) => s,
            Comment::DocBlock(_, s) => s,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SourceUnit(pub Vec<SourceUnitPart>);

impl Display for SourceUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Docable for SourceUnit {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::intersperse(self.0.iter().map(|x| x.to_doc()), Doc::hardline())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum SourceUnitPart {
    ContractDefinition(Box<ContractDefinition>),
    PragmaDirective(Loc, Identifier, StringLiteral),
    ImportDirective(Import),
    EnumDefinition(Box<EnumDefinition>),
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    ErrorDefinition(Box<ErrorDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    TypeDefinition(Box<TypeDefinition>),
    Using(Box<Using>),
    StraySemicolon(Loc),
}

impl Display for SourceUnitPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Docable for SourceUnitPart {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            SourceUnitPart::ContractDefinition(cd) => cd.to_doc(),
            SourceUnitPart::PragmaDirective(_, id, string) => RcDoc::text("pragma ")
                .append(id.to_doc())
                .append(RcDoc::space())
                .append(string.string.clone())
                .append(";")
                .append(RcDoc::hardline()),
            SourceUnitPart::ImportDirective(import) => import.to_doc().append(";"),
            SourceUnitPart::EventDefinition(ed) => ed.to_doc().append(";"),
            SourceUnitPart::ErrorDefinition(ed) => ed.to_doc().append(";"),
            SourceUnitPart::EnumDefinition(ed) => ed.to_doc(),
            _ => panic!("Unsupported source unit part: {:#?}", self),
        }
    }
}

impl SourceUnitPart {
    pub fn loc(&self) -> &Loc {
        match self {
            SourceUnitPart::ContractDefinition(def) => &def.loc,
            SourceUnitPart::PragmaDirective(loc, _, _) => loc,
            SourceUnitPart::ImportDirective(import) => import.loc(),
            SourceUnitPart::EnumDefinition(def) => &def.loc,
            SourceUnitPart::StructDefinition(def) => &def.loc,
            SourceUnitPart::EventDefinition(def) => &def.loc,
            SourceUnitPart::ErrorDefinition(def) => &def.loc,
            SourceUnitPart::FunctionDefinition(def) => &def.loc,
            SourceUnitPart::VariableDefinition(def) => &def.loc,
            SourceUnitPart::TypeDefinition(def) => &def.loc,
            SourceUnitPart::Using(def) => &def.loc,
            SourceUnitPart::StraySemicolon(loc) => loc,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Import {
    Plain(StringLiteral, Loc),
    GlobalSymbol(StringLiteral, Identifier, Loc),
    Rename(StringLiteral, Vec<(Identifier, Option<Identifier>)>, Loc),
}

impl Docable for Import {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Import::Plain(string, _) => RcDoc::text("import")
                .append(RcDoc::space())
                .append(string.to_doc()),
            Import::GlobalSymbol(string, id, _) => RcDoc::text("import")
                .append(RcDoc::space())
                .append(string.to_doc())
                .append(RcDoc::space())
                .append(" as ")
                .append(id.to_doc()),
            Import::Rename(string, imports, _) => RcDoc::text("import")
                .append(" { ")
                .append(RcDoc::intersperse(
                    imports.iter().map(|import| {
                        if import.1.is_some() {
                            import
                                .0
                                .to_doc()
                                .append(RcDoc::text(" as "))
                                .append(import.1.as_ref().unwrap().to_doc())
                        } else {
                            import.0.to_doc()
                        }
                    }),
                    RcDoc::text(", "),
                ))
                .append(" }")
                .append(" from ")
                .append(string.to_doc()),
        }
    }
}

impl Import {
    pub fn loc(&self) -> &Loc {
        match self {
            Import::Plain(_, loc) => loc,
            Import::GlobalSymbol(_, _, loc) => loc,
            Import::Rename(_, _, loc) => loc,
        }
    }
}

pub type ParameterList = Vec<(Loc, Option<Parameter>)>;

pub fn param_list_to_doc(ps: &ParameterList) -> RcDoc<()> {
    text!("(")
        .append(RcDoc::intersperse(
            ps.iter().map(|x| option_to_doc(&x.1)),
            RcDoc::text(",").append(Doc::space()),
        ))
        .append(")")
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Address,
    AddressPayable,
    Payable,
    Bool,
    String,
    Int(u16),
    Uint(u16),
    Bytes(u8),
    Rational,
    DynamicBytes,
    Mapping(Loc, Box<Expression>, Box<Expression>),
    Function {
        params: Vec<(Loc, Option<Parameter>)>,
        attributes: Vec<FunctionAttribute>,
        returns: Option<(ParameterList, Vec<FunctionAttribute>)>,
    },
}

impl Docable for Type {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Type::Address => text!("address"),
            Type::AddressPayable => text!("address payable"),
            Type::Payable => text!("payable"),
            Type::Bool => text!("bool"),
            Type::String => text!("string"),
            Type::Int(size) => text!("int").append(size.to_string()),
            Type::Uint(size) => text!("uint").append(size.to_string()),
            Type::Bytes(size) => text!("bytes").append(size.to_string()),
            Type::DynamicBytes => text!("bytes"),
            Type::Mapping(_, from_expr, to_expr) => text!("mapping(")
                .append(from_expr.to_doc())
                .append(text!(" => "))
                .append(to_expr.to_doc())
                .append(text!(")")),
            _ => panic!("{:#?}", self),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum StorageLocation {
    Memory(Loc),
    Storage(Loc),
    Calldata(Loc),
}

impl Docable for StorageLocation {
    fn to_doc(&self) -> RcDoc<()> {
        text!(self.to_string())
    }
}

impl CodeLocation for StorageLocation {
    fn loc(&self) -> Loc {
        match self {
            StorageLocation::Memory(l)
            | StorageLocation::Storage(l)
            | StorageLocation::Calldata(l) => *l,
        }
    }
}

impl fmt::Display for StorageLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageLocation::Memory(_) => write!(f, "memory"),
            StorageLocation::Storage(_) => write!(f, "storage"),
            StorageLocation::Calldata(_) => write!(f, "calldata"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct VariableDeclaration {
    pub loc: Loc,
    pub ty: Expression,
    pub storage: Option<StorageLocation>,
    pub name: Identifier,
}

impl Docable for VariableDeclaration {
    fn to_doc(&self) -> RcDoc<()> {
        self.ty
            .to_doc()
            .append(RcDoc::space())
            .append(option_space_to_doc(&self.storage))
            .append(self.name.to_doc())
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::vec_box)]
pub struct StructDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<VariableDeclaration>,
}

impl Docable for StructDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        text!("struct ")
            .append(self.name.to_doc())
            .append(" {")
            .append(RcDoc::intersperse(
                self.fields
                    .iter()
                    .map(|field| RcDoc::hardline().append(field.to_doc().append(";")).nest(4)),
                RcDoc::nil(),
            ))
            .append(RcDoc::hardline())
            .append("}")
    }
}

impl<'a> Hash for &'a StructDefinition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(*self, state)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ContractPart {
    StructDefinition(Box<StructDefinition>),
    EventDefinition(Box<EventDefinition>),
    EnumDefinition(Box<EnumDefinition>),
    ErrorDefinition(Box<ErrorDefinition>),
    VariableDefinition(Box<VariableDefinition>),
    FunctionDefinition(Box<FunctionDefinition>),
    TypeDefinition(Box<TypeDefinition>),
    StraySemicolon(Loc),
    Using(Box<Using>),
}

impl Docable for ContractPart {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            ContractPart::EventDefinition(ed) => ed.to_doc().append(";"),
            ContractPart::FunctionDefinition(fd) => fd.to_doc(),
            ContractPart::VariableDefinition(vd) => vd.to_doc().append(";"),
            ContractPart::EnumDefinition(ed) => ed.to_doc(),
            ContractPart::StructDefinition(sd) => sd.to_doc(),
            ContractPart::StraySemicolon(..) => text!(";"),
            ContractPart::Using(using) => using.to_doc().append(";"),
            ContractPart::ErrorDefinition(ed) => ed.to_doc().append(";"),
            ContractPart::TypeDefinition(td) => (**td).to_doc().append(";"),
        }
    }
}

impl ContractPart {
    // Return the location of the part. Note that this excluded the body of the function
    pub fn loc(&self) -> &Loc {
        match self {
            ContractPart::StructDefinition(def) => &def.loc,
            ContractPart::EventDefinition(def) => &def.loc,
            ContractPart::EnumDefinition(def) => &def.loc,
            ContractPart::ErrorDefinition(def) => &def.loc,
            ContractPart::VariableDefinition(def) => &def.loc,
            ContractPart::FunctionDefinition(def) => &def.loc,
            ContractPart::TypeDefinition(def) => &def.loc,
            ContractPart::StraySemicolon(loc) => loc,
            ContractPart::Using(def) => &def.loc,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum UsingList {
    Library(IdentifierPath),
    Functions(Vec<IdentifierPath>),
}

impl Docable for UsingList {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            UsingList::Library(ip) => ip.to_doc(),
            UsingList::Functions(ips) => text!("{").append(list_to_doc(ips)).append("}"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Using {
    pub loc: Loc,
    pub list: UsingList,
    pub ty: Option<Expression>,
    pub global: Option<Identifier>,
}

impl Docable for Using {
    fn to_doc(&self) -> RcDoc<()> {
        assert!(self.global.is_none());
        assert!(self.ty.is_some());
        text!("using ")
            .append(self.list.to_doc())
            .append(" for ")
            .append(option_to_doc(&self.ty))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ContractTy {
    Abstract(Loc),
    Contract(Loc),
    Interface(Loc),
    Library(Loc),
}

impl Docable for ContractTy {
    fn to_doc(&self) -> RcDoc<()> {
        text!(self.to_string())
    }
}

impl fmt::Display for ContractTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractTy::Abstract(_) => write!(f, "abstract contract"),
            ContractTy::Contract(_) => write!(f, "contract"),
            ContractTy::Interface(_) => write!(f, "interface"),
            ContractTy::Library(_) => write!(f, "library"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Base {
    pub loc: Loc,
    pub name: IdentifierPath,
    pub args: Option<Vec<Expression>>,
}

impl Docable for Base {
    fn to_doc(&self) -> RcDoc<()> {
        let args = tern!(
            self.args.is_some(),
            paren_list_to_doc(self.args.as_ref().unwrap()),
            RcDoc::nil()
        );
        self.name.to_doc().append(args)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ContractDefinition {
    pub loc: Loc,
    pub ty: ContractTy,
    pub name: Identifier,
    pub base: Vec<Base>,
    pub parts: Vec<ContractPart>,
}

impl Docable for ContractDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text("contract")
            .append(RcDoc::space())
            .append(self.name.to_doc())
            .append(RcDoc::space())
            .append(tern!(self.base.is_empty(), RcDoc::nil(), text!(" is ")))
            .append(list_to_doc(&self.base))
            .append(space_if(!self.base.is_empty()))
            .append(RcDoc::text("{"))
            .append(RcDoc::intersperse(
                self.parts
                    .iter()
                    .map(|x| RcDoc::hardline().append(x.to_doc()).nest(4)),
                Doc::hardline(),
            ))
            .append(RcDoc::line())
            .append(RcDoc::text("}"))
    }
}

impl fmt::Display for ContractDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl<'a> Eq for &'a ContractDefinition {}

impl<'a> Hash for &'a ContractDefinition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(*self, state)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct EventParameter {
    pub ty: Expression,
    pub loc: Loc,
    pub indexed: bool,
    pub name: Option<Identifier>,
}

impl Docable for EventParameter {
    fn to_doc(&self) -> RcDoc<()> {
        self.ty
            .to_doc()
            .append(RcDoc::space())
            .append(tern!(self.indexed, text!("indexed "), RcDoc::nil()))
            .append(option_to_doc(&self.name))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct EventDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<EventParameter>,
    pub anonymous: bool,
}

impl<'a> Hash for &'a EventDefinition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(*self, state)
    }
}

impl Docable for EventDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text("event")
            .append(RcDoc::space())
            .append(self.name.to_doc())
            .append(RcDoc::space())
            .append(paren_list_to_doc(&self.fields))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ErrorParameter {
    pub ty: Expression,
    pub loc: Loc,
    pub name: Option<Identifier>,
}

impl Docable for ErrorParameter {
    fn to_doc(&self) -> RcDoc<()> {
        self.ty
            .to_doc()
            .append(RcDoc::space())
            .append(option_to_doc(&self.name))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ErrorDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub fields: Vec<ErrorParameter>,
}

impl Docable for ErrorDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text("error")
            .append(RcDoc::space())
            .append(self.name.to_doc())
            .append(paren_list_to_doc(&self.fields))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub values: Vec<Identifier>,
}

impl Docable for EnumDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        text!("enum")
            .append(RcDoc::space())
            .append(self.name.to_doc())
            .append(RcDoc::space())
            .append("{")
            .append(indent_list_to_doc(&self.values))
            .append("}")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum VariableAttribute {
    Visibility(Visibility),
    Constant(Loc),
    Immutable(Loc),
    Override(Loc, Vec<IdentifierPath>),
}

impl Docable for VariableAttribute {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            VariableAttribute::Visibility(vis) => vis.to_doc(),
            VariableAttribute::Constant(..) => text!("constant"),
            VariableAttribute::Immutable(..) => text!("immutable"),
            _ => panic!("Not supported: {:#?}", self),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct VariableDefinition {
    pub loc: Loc,
    pub ty: Expression,
    pub attrs: Vec<VariableAttribute>,
    pub name: Identifier,
    pub initializer: Option<Expression>,
}

impl Docable for VariableDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        let attrs = tern!(
            self.attrs.is_empty(),
            RcDoc::nil(),
            RcDoc::space().append(spaced_list_to_doc(&self.attrs))
        );
        let equal = tern!(self.initializer.is_some(), text!(" = "), RcDoc::nil());
        let init = option_to_doc(&self.initializer);

        self.ty
            .to_doc()
            .append(attrs)
            .append(RcDoc::space())
            .append(self.name.to_doc())
            .append(equal)
            .append(init)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeDefinition {
    pub loc: Loc,
    pub name: Identifier,
    pub ty: Expression,
}

impl Docable for TypeDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        text!("type ")
            .append(self.name.to_doc())
            .append(" is ")
            .append(self.ty.to_doc())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct StringLiteral {
    pub loc: Loc,
    pub unicode: bool,
    pub string: String,
}

impl Docable for StringLiteral {
    fn to_doc(&self) -> RcDoc<()> {
        assert!(!self.unicode);
        text!("\"").append(self.string.clone()).append("\"")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct HexLiteral {
    pub loc: Loc,
    pub hex: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NamedArgument {
    pub loc: Loc,
    pub name: Identifier,
    pub expr: Expression,
}

impl Docable for NamedArgument {
    fn to_doc(&self) -> RcDoc<()> {
        self.name
            .to_doc()
            .append(text!(": "))
            .append(self.expr.to_doc())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
    Seconds(Loc),
    Minutes(Loc),
    Hours(Loc),
    Days(Loc),
    Weeks(Loc),
    Wei(Loc),
    Gwei(Loc),
    Ether(Loc),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    PostIncrement(Loc, Box<Expression>),
    PostDecrement(Loc, Box<Expression>),
    New(Loc, Box<Expression>),
    ArraySubscript(Loc, Box<Expression>, Option<Box<Expression>>),
    ArraySlice(
        Loc,
        Box<Expression>,
        Option<Box<Expression>>,
        Option<Box<Expression>>,
    ),
    Parenthesis(Loc, Box<Expression>),
    MemberAccess(Loc, Box<Expression>, Identifier),
    FunctionCall(Loc, Box<Expression>, Vec<Expression>),
    FunctionCallBlock(Loc, Box<Expression>, Box<Statement>),
    NamedFunctionCall(Loc, Box<Expression>, Vec<NamedArgument>),
    Not(Loc, Box<Expression>),
    Complement(Loc, Box<Expression>),
    Delete(Loc, Box<Expression>),
    PreIncrement(Loc, Box<Expression>),
    PreDecrement(Loc, Box<Expression>),
    UnaryPlus(Loc, Box<Expression>),
    UnaryMinus(Loc, Box<Expression>),
    Power(Loc, Box<Expression>, Box<Expression>),
    Multiply(Loc, Box<Expression>, Box<Expression>),
    Divide(Loc, Box<Expression>, Box<Expression>),
    Modulo(Loc, Box<Expression>, Box<Expression>),
    Add(Loc, Box<Expression>, Box<Expression>),
    Subtract(Loc, Box<Expression>, Box<Expression>),
    ShiftLeft(Loc, Box<Expression>, Box<Expression>),
    ShiftRight(Loc, Box<Expression>, Box<Expression>),
    BitwiseAnd(Loc, Box<Expression>, Box<Expression>),
    BitwiseXor(Loc, Box<Expression>, Box<Expression>),
    BitwiseOr(Loc, Box<Expression>, Box<Expression>),
    Less(Loc, Box<Expression>, Box<Expression>),
    More(Loc, Box<Expression>, Box<Expression>),
    LessEqual(Loc, Box<Expression>, Box<Expression>),
    MoreEqual(Loc, Box<Expression>, Box<Expression>),
    Equal(Loc, Box<Expression>, Box<Expression>),
    NotEqual(Loc, Box<Expression>, Box<Expression>),
    And(Loc, Box<Expression>, Box<Expression>),
    Or(Loc, Box<Expression>, Box<Expression>),
    Ternary(Loc, Box<Expression>, Box<Expression>, Box<Expression>),
    Assign(Loc, Box<Expression>, Box<Expression>),
    AssignOr(Loc, Box<Expression>, Box<Expression>),
    AssignAnd(Loc, Box<Expression>, Box<Expression>),
    AssignXor(Loc, Box<Expression>, Box<Expression>),
    AssignShiftLeft(Loc, Box<Expression>, Box<Expression>),
    AssignShiftRight(Loc, Box<Expression>, Box<Expression>),
    AssignAdd(Loc, Box<Expression>, Box<Expression>),
    AssignSubtract(Loc, Box<Expression>, Box<Expression>),
    AssignMultiply(Loc, Box<Expression>, Box<Expression>),
    AssignDivide(Loc, Box<Expression>, Box<Expression>),
    AssignModulo(Loc, Box<Expression>, Box<Expression>),
    BoolLiteral(Loc, bool),
    NumberLiteral(Loc, String, String),
    RationalNumberLiteral(Loc, String, String, String),
    HexNumberLiteral(Loc, String),
    StringLiteral(Vec<StringLiteral>),
    Type(Loc, Type),
    HexLiteral(Vec<HexLiteral>),
    AddressLiteral(Loc, String),
    Variable(Identifier),
    List(Loc, ParameterList),
    ArrayLiteral(Loc, Vec<Expression>),
    Unit(Loc, Box<Expression>, Unit),
    This(Loc),
}

impl<'a> Eq for &'a Expression {}

impl<'a> Hash for &'a Expression {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(*self, state)
    }
}

impl Docable for Expression {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Expression::PostIncrement(_, expr) => expr.to_doc().append("++"),
            Expression::PostDecrement(_, expr) => expr.to_doc().append("--"),
            Expression::New(_, expr) => text!("new ").append(expr.to_doc()),
            Expression::ArraySubscript(_, expr, mexpr) => expr
                .to_doc()
                .append("[")
                .append(option_box_to_doc(mexpr))
                .append("]"),
            Expression::ArraySlice(..) => panic!("Array slice not supported: {:#?}", self),
            Expression::Parenthesis(_, expr) => text!("(").append(expr.to_doc()).append(")"),
            Expression::FunctionCall(_, fun, args) => fun.to_doc().append(paren_list_to_doc(args)),
            Expression::MemberAccess(_, contract, field) => {
                contract.to_doc().append(".").append(field.to_doc())
            }
            Expression::Not(_, expr) => text!("!").append(expr.to_doc()),
            Expression::PreIncrement(_, expr) => text!("++").append(expr.to_doc()),
            Expression::PreDecrement(_, expr) => text!("--").append(expr.to_doc()),
            Expression::Assign(_, lhs, rhs) => lhs.bin_op_doc("=", rhs),
            Expression::AssignAdd(_, lhs, rhs) => lhs.bin_op_doc("+=", rhs),
            Expression::AssignSubtract(_, lhs, rhs) => lhs.bin_op_doc("-=", rhs),
            Expression::AssignMultiply(_, lhs, rhs) => lhs.bin_op_doc("*=", rhs),
            Expression::AssignDivide(_, lhs, rhs) => lhs.bin_op_doc("/=", rhs),
            Expression::AssignModulo(_, lhs, rhs) => lhs.bin_op_doc("%=", rhs),
            Expression::Multiply(_, left, right) => left.bin_op_doc("*", right),
            Expression::Divide(_, left, right) => left.bin_op_doc("/", right),
            Expression::Modulo(_, left, right) => left.bin_op_doc("%", right),
            Expression::Add(_, left, right) => left.bin_op_doc("+", right),
            Expression::Subtract(_, left, right) => left.bin_op_doc("-", right),
            Expression::ShiftLeft(_, left, right) => left.bin_op_doc("<<", right),
            Expression::ShiftRight(_, left, right) => left.bin_op_doc(">>", right),
            Expression::Less(_, left, right) => left.bin_op_doc("<", right),
            Expression::More(_, left, right) => left.bin_op_doc(">", right),
            Expression::LessEqual(_, left, right) => left.bin_op_doc("<=", right),
            Expression::MoreEqual(_, left, right) => left.bin_op_doc(">=", right),
            Expression::Equal(_, left, right) => left.bin_op_doc("==", right),
            Expression::NotEqual(_, left, right) => left.bin_op_doc("!=", right),
            Expression::And(_, left, right) => left.bin_op_doc("&&", right),
            Expression::Or(_, left, right) => left.bin_op_doc("||", right),
            Expression::NumberLiteral(_, num, _) => text!(num),
            Expression::ArrayLiteral(_, elems) => text!("[").append(list_to_doc(elems)).append("]"),
            Expression::Type(_, ty) => ty.to_doc(),
            Expression::Variable(id) => id.to_doc(),
            Expression::This(..) => text!("this"),
            Expression::List(_, ps) => param_list_to_doc(ps),
            Expression::StringLiteral(lits) => list_to_doc(lits),
            Expression::BoolLiteral(_, blit) => text!(blit.to_string()),
            Expression::FunctionCallBlock(_, base, expr) => {
                base.to_doc().append("{").append(expr.to_doc()).append("}")
            }
            _ => panic!("{:#?}", self),
        }
    }
}

impl Expression {
    fn bin_op_doc<'a>(&'a self, op: &'a str, right: &'a Expression) -> RcDoc<'a> {
        self.to_doc()
            .append(RcDoc::space())
            .append(op.to_string())
            .append(RcDoc::space())
            .append(right.to_doc())
    }
}

impl CodeLocation for Expression {
    fn loc(&self) -> Loc {
        match self {
            Expression::PostIncrement(loc, _)
            | Expression::PostDecrement(loc, _)
            | Expression::New(loc, _)
            | Expression::Parenthesis(loc, _)
            | Expression::ArraySubscript(loc, ..)
            | Expression::ArraySlice(loc, ..)
            | Expression::MemberAccess(loc, ..)
            | Expression::FunctionCall(loc, ..)
            | Expression::FunctionCallBlock(loc, ..)
            | Expression::NamedFunctionCall(loc, ..)
            | Expression::Not(loc, _)
            | Expression::Complement(loc, _)
            | Expression::Delete(loc, _)
            | Expression::PreIncrement(loc, _)
            | Expression::PreDecrement(loc, _)
            | Expression::UnaryPlus(loc, _)
            | Expression::UnaryMinus(loc, _)
            | Expression::Power(loc, ..)
            | Expression::Multiply(loc, ..)
            | Expression::Divide(loc, ..)
            | Expression::Modulo(loc, ..)
            | Expression::Add(loc, ..)
            | Expression::Subtract(loc, ..)
            | Expression::ShiftLeft(loc, ..)
            | Expression::ShiftRight(loc, ..)
            | Expression::BitwiseAnd(loc, ..)
            | Expression::BitwiseXor(loc, ..)
            | Expression::BitwiseOr(loc, ..)
            | Expression::Less(loc, ..)
            | Expression::More(loc, ..)
            | Expression::LessEqual(loc, ..)
            | Expression::MoreEqual(loc, ..)
            | Expression::Equal(loc, ..)
            | Expression::NotEqual(loc, ..)
            | Expression::And(loc, ..)
            | Expression::Or(loc, ..)
            | Expression::Ternary(loc, ..)
            | Expression::Assign(loc, ..)
            | Expression::AssignOr(loc, ..)
            | Expression::AssignAnd(loc, ..)
            | Expression::AssignXor(loc, ..)
            | Expression::AssignShiftLeft(loc, ..)
            | Expression::AssignShiftRight(loc, ..)
            | Expression::AssignAdd(loc, ..)
            | Expression::AssignSubtract(loc, ..)
            | Expression::AssignMultiply(loc, ..)
            | Expression::AssignDivide(loc, ..)
            | Expression::AssignModulo(loc, ..)
            | Expression::BoolLiteral(loc, _)
            | Expression::NumberLiteral(loc, ..)
            | Expression::RationalNumberLiteral(loc, ..)
            | Expression::HexNumberLiteral(loc, _)
            | Expression::ArrayLiteral(loc, _)
            | Expression::List(loc, _)
            | Expression::Type(loc, _)
            | Expression::Unit(loc, ..)
            | Expression::This(loc)
            | Expression::Variable(Identifier { loc, .. })
            | Expression::AddressLiteral(loc, _) => *loc,
            Expression::StringLiteral(v) => v[0].loc,
            Expression::HexLiteral(v) => v[0].loc,
        }
    }
}

impl Expression {
    pub fn remove_parenthesis(&self) -> &Expression {
        if let Expression::Parenthesis(_, expr) = self {
            expr
        } else {
            self
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub loc: Loc,
    pub ty: Expression,
    pub storage: Option<StorageLocation>,
    pub name: Option<Identifier>,
}

impl Docable for Parameter {
    fn to_doc(&self) -> RcDoc<()> {
        self.ty
            .to_doc()
            .append(space_if(self.storage.is_some()))
            .append(option_to_doc(&self.storage))
            .append(space_if(self.name.is_some()))
            .append(option_to_doc(&self.name))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Mutability {
    Pure(Loc),
    View(Loc),
    Constant(Loc),
    Payable(Loc),
}

impl Docable for Mutability {
    fn to_doc(&self) -> RcDoc<()> {
        text!(self.to_string())
    }
}

impl fmt::Display for Mutability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mutability::Pure(_) => write!(f, "pure"),
            Mutability::Constant(_) | Mutability::View(_) => write!(f, "view"),
            Mutability::Payable(_) => write!(f, "payable"),
        }
    }
}

impl CodeLocation for Mutability {
    fn loc(&self) -> Loc {
        match self {
            Mutability::Pure(loc)
            | Mutability::Constant(loc)
            | Mutability::View(loc)
            | Mutability::Payable(loc) => *loc,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Visibility {
    External(Option<Loc>),
    Public(Option<Loc>),
    Internal(Option<Loc>),
    Private(Option<Loc>),
}

impl Docable for Visibility {
    fn to_doc(&self) -> RcDoc<()> {
        text!(self.to_string())
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public(_) => write!(f, "public"),
            Visibility::External(_) => write!(f, "external"),
            Visibility::Internal(_) => write!(f, "internal"),
            Visibility::Private(_) => write!(f, "private"),
        }
    }
}

impl OptionalCodeLocation for Visibility {
    fn loc(&self) -> Option<Loc> {
        match self {
            Visibility::Public(loc)
            | Visibility::External(loc)
            | Visibility::Internal(loc)
            | Visibility::Private(loc) => *loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionAttribute {
    Mutability(Mutability),
    Visibility(Visibility),
    Virtual(Loc),
    Immutable(Loc),
    Override(Loc, Vec<IdentifierPath>),
    BaseOrModifier(Loc, Base),
}

impl Docable for FunctionAttribute {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            FunctionAttribute::Virtual(..) => text!("virtual"),
            FunctionAttribute::Immutable(..) => text!("immutable"),
            FunctionAttribute::Mutability(mutability) => mutability.to_doc(),
            FunctionAttribute::Visibility(visibility) => visibility.to_doc(),
            FunctionAttribute::Override(_, ids) => text!("override ")
                .append(tern!(!ids.is_empty(), text!("("), RcDoc::nil()))
                .append(list_to_doc(ids))
                .append(tern!(!ids.is_empty(), text!(")"), RcDoc::nil())),
            FunctionAttribute::BaseOrModifier(_, base) => base.to_doc(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FunctionTy {
    Constructor,
    Function,
    Fallback,
    Receive,
    Modifier,
}

impl Docable for FunctionTy {
    fn to_doc(&self) -> RcDoc<()> {
        text!(self.to_string())
    }
}

impl fmt::Display for FunctionTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionTy::Constructor => write!(f, "constructor"),
            FunctionTy::Function => write!(f, "function"),
            FunctionTy::Fallback => write!(f, "fallback"),
            FunctionTy::Receive => write!(f, "receive"),
            FunctionTy::Modifier => write!(f, "modifier"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionDefinition {
    pub loc: Loc,
    pub ty: FunctionTy,
    pub name: Option<Identifier>,
    pub name_loc: Loc,
    pub params: ParameterList,
    pub attributes: Vec<FunctionAttribute>,
    pub return_not_returns: Option<Loc>,
    pub returns: ParameterList,
    pub body: Option<Statement>,
}

impl Docable for FunctionDefinition {
    fn to_doc(&self) -> RcDoc<()> {
        assert!(self.name.is_some() || self.ty == FunctionTy::Constructor);
        let name = self
            .ty
            .to_doc()
            .append(RcDoc::space())
            .append(option_to_doc(&self.name));
        let returns = tern!(
            self.returns.is_empty(),
            RcDoc::nil(),
            text!("returns ").append(param_list_to_doc(&self.returns))
        );
        name.append(param_list_to_doc(&self.params))
            .append(RcDoc::space())
            .append(spaced_list_to_doc(&self.attributes))
            .append(RcDoc::space())
            .append(returns)
            .append(self.body.as_ref().unwrap().to_doc())
    }
}

impl<'a> Eq for &'a FunctionDefinition {}

impl<'a> Hash for &'a FunctionDefinition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(*self, state)
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::large_enum_variant, clippy::type_complexity)]
pub enum Statement {
    Block {
        loc: Loc,
        unchecked: bool,
        statements: Vec<Statement>,
    },
    Assembly {
        loc: Loc,
        dialect: Option<StringLiteral>,
        flags: Option<Vec<StringLiteral>>,
        block: YulBlock,
    },
    Args(Loc, Vec<NamedArgument>),
    If(Loc, Expression, Box<Statement>, Option<Box<Statement>>),
    While(Loc, Expression, Box<Statement>),
    Expression(Loc, Expression),
    VariableDefinition(Loc, VariableDeclaration, Option<Expression>),
    For(
        Loc,
        Option<Box<Statement>>,
        Option<Box<Expression>>,
        Option<Box<Statement>>,
        Option<Box<Statement>>,
    ),
    DoWhile(Loc, Box<Statement>, Expression),
    Continue(Loc),
    Break(Loc),
    Return(Loc, Option<Expression>),
    Revert(Loc, Option<IdentifierPath>, Vec<Expression>),
    RevertNamedArgs(Loc, Option<IdentifierPath>, Vec<NamedArgument>),
    Emit(Loc, Expression),
    Try(
        Loc,
        Expression,
        Option<(ParameterList, Box<Statement>)>,
        Vec<CatchClause>,
    ),
}

impl Docable for Statement {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Statement::Block { statements, .. } => text!("{")
                .append(RcDoc::intersperse(
                    statements
                        .iter()
                        .map(|x| RcDoc::hardline().append(x.to_doc()).nest(4)),
                    Doc::nil(),
                ))
                .append(RcDoc::hardline())
                .append("}"),
            Statement::Assembly { .. } => panic!("Assembly printing not supported"),
            Statement::Args(_, args) => spaced_list_to_doc(args),
            Statement::If(_, cond, tb, fb) => {
                let fdoc = tern!(
                    fb.is_some(),
                    text!(" else ").append(option_box_to_doc(fb)),
                    RcDoc::nil()
                );
                text!("if (")
                    .append(cond.to_doc())
                    .append(")")
                    .append(RcDoc::space())
                    .append(tb.to_doc())
                    .append(fdoc)
            }
            Statement::While(_, cond, body) => text!("while (")
                .append(cond.to_doc())
                .append(")")
                .append(RcDoc::space())
                .append(body.to_doc()),
            Statement::Expression(_, expr) => expr.to_doc().append(";"),
            Statement::VariableDefinition(_, decl, None) => decl.to_doc().append(";"),
            Statement::VariableDefinition(_, decl, Some(expr)) => decl
                .to_doc()
                .append(" = ")
                .append(expr.to_doc())
                .append(";"),
            Statement::Continue(..) => text!("continue;"),
            Statement::Break(..) => text!("break;"),
            Statement::Return(_, Some(expr)) => text!("return ").append(expr.to_doc()).append(";"),
            Statement::Return(_, None) => text!("return;"),
            Statement::Revert(_, id, exprs) => text!("revert ")
                .append(option_to_doc(id))
                .append(paren_list_to_doc(exprs))
                .append(";"),
            Statement::RevertNamedArgs(..) => panic!("Revert named args printing not supported"),
            Statement::Emit(_, expr) => text!("emit ").append(expr.to_doc()).append(";"),
            Statement::Try(..) => panic!("Try printing not supported"),
            _ => panic!("{:#?}", self),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CatchClause {
    Simple(Loc, Option<Parameter>, Statement),
    Named(Loc, Identifier, Parameter, Statement),
}

#[derive(Debug, PartialEq, Clone)]
pub enum YulStatement {
    Assign(Loc, Vec<YulExpression>, YulExpression),
    VariableDeclaration(Loc, Vec<YulTypedIdentifier>, Option<YulExpression>),
    If(Loc, YulExpression, YulBlock),
    For(YulFor),
    Switch(YulSwitch),
    Leave(Loc),
    Break(Loc),
    Continue(Loc),
    Block(YulBlock),
    FunctionDefinition(Box<YulFunctionDefinition>),
    FunctionCall(Box<YulFunctionCall>),
}
#[derive(PartialEq, Clone, Debug)]
pub struct YulSwitch {
    pub loc: Loc,
    pub condition: YulExpression,
    pub cases: Vec<YulSwitchOptions>,
    pub default: Option<YulSwitchOptions>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct YulFor {
    pub loc: Loc,
    pub init_block: YulBlock,
    pub condition: YulExpression,
    pub post_block: YulBlock,
    pub execution_block: YulBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulBlock {
    pub loc: Loc,
    pub statements: Vec<YulStatement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum YulExpression {
    BoolLiteral(Loc, bool, Option<Identifier>),
    NumberLiteral(Loc, String, String, Option<Identifier>),
    HexNumberLiteral(Loc, String, Option<Identifier>),
    HexStringLiteral(HexLiteral, Option<Identifier>),
    StringLiteral(StringLiteral, Option<Identifier>),
    Variable(Identifier),
    FunctionCall(Box<YulFunctionCall>),
    SuffixAccess(Loc, Box<YulExpression>, Identifier),
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulTypedIdentifier {
    pub loc: Loc,
    pub id: Identifier,
    pub ty: Option<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulFunctionDefinition {
    pub loc: Loc,
    pub id: Identifier,
    pub params: Vec<YulTypedIdentifier>,
    pub returns: Vec<YulTypedIdentifier>,
    pub body: YulBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct YulFunctionCall {
    pub loc: Loc,
    pub id: Identifier,
    pub arguments: Vec<YulExpression>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum YulSwitchOptions {
    Case(Loc, YulExpression, YulBlock),
    Default(Loc, YulBlock),
}

impl CodeLocation for YulSwitchOptions {
    fn loc(&self) -> Loc {
        match self {
            YulSwitchOptions::Case(loc, ..) | YulSwitchOptions::Default(loc, ..) => *loc,
        }
    }
}

impl CodeLocation for Statement {
    fn loc(&self) -> Loc {
        match self {
            Statement::Block { loc, .. }
            | Statement::Assembly { loc, .. }
            | Statement::Args(loc, ..)
            | Statement::If(loc, ..)
            | Statement::While(loc, ..)
            | Statement::Expression(loc, ..)
            | Statement::VariableDefinition(loc, ..)
            | Statement::For(loc, ..)
            | Statement::DoWhile(loc, ..)
            | Statement::Continue(loc)
            | Statement::Break(loc)
            | Statement::Return(loc, ..)
            | Statement::Revert(loc, ..)
            | Statement::RevertNamedArgs(loc, ..)
            | Statement::Emit(loc, ..)
            | Statement::Try(loc, ..) => *loc,
        }
    }
}

impl YulStatement {
    pub fn loc(&self) -> Loc {
        match self {
            YulStatement::Assign(loc, ..)
            | YulStatement::VariableDeclaration(loc, ..)
            | YulStatement::If(loc, ..)
            | YulStatement::Leave(loc, ..)
            | YulStatement::Break(loc, ..)
            | YulStatement::Continue(loc, ..) => *loc,

            YulStatement::Block(block) => block.loc,

            YulStatement::FunctionDefinition(func_def) => func_def.loc,

            YulStatement::FunctionCall(func_call) => func_call.loc,

            YulStatement::For(for_struct) => for_struct.loc,
            YulStatement::Switch(switch_struct) => switch_struct.loc,
        }
    }
}

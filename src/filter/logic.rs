type BoxedElement = Box<dyn crate::elements::Element + Send + Sync>;

pub enum Element {
    Modifiable(BoxedElement),
    Committed(BoxedElement),
    None,
}

pub enum ElementType {
    Node,
    Way,
    Relation
}

pub struct Equals {
    pub key: String,
    pub value: String,
}

pub enum Modifier {
    Set,
    Rename,
    Delete,
}

pub enum SelectorStatement {
    Type {
        elementtype: ElementType,
    },
    Has {
        key: String,
    },
    Equals {
        key: String,
        value: String,
    },
}

impl SelectorStatement {
    fn test(self, element: Element) -> bool {
        match self {
            SelectorStatement::Type { elementtype } => {
                unimplemented!();
            },
            SelectorStatement::Has { key } => {
                unimplemented!();
            },
            SelectorStatement::Equals { key, value } => {
                unimplemented!();
            },
        }
    }
}

pub enum Statement {
    CommitStatement,
    DropStatement,
    DeleteStatement {
        key: String,
    },
    SetStatement {
        key: String,
        value: String,
    },
    RenameStatement {
        old_key: String,
        new_key: String,
    },
    SelectionBlock {
        selector: SelectorStatement,
        statements: Vec<Statement>,
    },
}

impl Statement {
    fn evaluate(self, element: Element) -> Element {
        match element {
            Element::Modifiable(e) => {
                match self {
                    Statement::CommitStatement => Element::Committed(e),
                    Statement::DropStatement => Element::None,
                    Statement::DeleteStatement { key } => {
                        unimplemented!();
                    },
                    Statement::SetStatement { key, value } => {
                        unimplemented!();
                    },
                    Statement::RenameStatement { old_key, new_key } => {
                        unimplemented!();
                    },
                    Statement::SelectionBlock { selector, statements } => {
                        unimplemented!();
                    },
                }
            },
            e => e,
        }
    }
}

pub struct Filter {
    pub statements: Vec<Statement>
}

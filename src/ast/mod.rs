use tokens;

#[derive(Debug, Clone)]
pub struct Spec<'a> {
    items: Vec<Item<'a>>,
}

#[derive(Debug, Clone)]
pub struct Item<'a> {
    params: Vec<Param<'a>>,
    template: Vec<Match<'a>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Param<'a> {
    key: &'a str,
    value: &'a str,
}

#[derive(Debug, Copy, Clone)]
pub enum Match<'a> {
    MultipleLines,
    Text(&'a str),
    Var(&'a str),
}

impl<'a> Spec<'a> {

}
#[macro_use] extern crate rocket;

#[derive(FromForm)]
enum Thing { }

#[derive(FromForm)]
struct Foo1;

#[derive(FromForm)]
struct Foo2 {  }

#[derive(FromForm)]
struct Foo3(usize);

#[derive(FromForm)]
struct NextTodoTask<'f, 'a> {
    description: String,
    raw_description: &'f str,
    other: &'a str,
    completed: bool,
}

#[derive(FromForm)]
struct BadName1 {
    #[field(name = "isindex")]
    field: String,
}

#[derive(FromForm)]
struct Demo2 {
    #[field(name = "foo")]
    field: String,
    foo: usize,
}

#[derive(FromForm)]
struct MyForm9 {
    #[field(name = "hello")]
    first: String,
    #[field(name = "hello")]
    other: String,
}

#[derive(FromForm)]
struct MyForm10 {
    first: String,
    #[field(name = "first")]
    other: String,
}

#[derive(FromForm)]
struct MyForm {
    #[field(name = "blah", field = "bloo")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm1 {
    #[field]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm2 {
    #[field("blah")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm3 {
    #[field(123)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm4 {
    #[field(beep = "bop")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm5 {
    #[field(name = "blah")]
    #[field(name = "blah")]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm6 {
    #[field(name = true)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm7 {
    #[field(name)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm8 {
    #[field(name = 123)]
    my_field: String,
}

#[derive(FromForm)]
struct MyForm11 {
    #[field(name = "hello&world")]
    first: String,
}

#[derive(FromForm)]
struct MyForm12 {
    #[field(name = "!@#$%^&*()_")]
    first: String,
}

#[derive(FromForm)]
struct MyForm13 {
    #[field(name = "?")]
    first: String,
}

#[derive(FromForm)]
struct MyForm14 {
    #[field(name = "")]
    first: String,
}

#[derive(FromForm)]
struct BadName2 {
    #[field(name = "a&b")]
    field: String,
}

#[derive(FromForm)]
struct BadName3 {
    #[field(name = "a=")]
    field: String,
}

fn main() { }

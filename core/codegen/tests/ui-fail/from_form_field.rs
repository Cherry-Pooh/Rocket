#[macro_use] extern crate rocket;

#[derive(FromFormField)]
struct Foo1;

#[derive(FromFormField)]
struct Foo2(usize);

#[derive(FromFormField)]
struct Foo3 {
    foo: usize,
}

#[derive(FromFormField)]
enum Foo4 {
    A(usize),
}

#[derive(FromFormField)]
enum Foo5 { }

#[derive(FromFormField)]
enum Foo6<T> {
    A(T),
}

#[derive(FromFormField)]
enum Bar1 {
    #[field(value = 123)]
    A,
}

#[derive(FromFormField)]
enum Bar2 {
    #[field(value)]
    A,
}

#[derive(FromForm)]
struct Renamed0 {
    #[field(name = "foo")]
    #[field(name = uncased("FOO"))]
    single: usize,
}

#[derive(FromForm)]
struct Renamed1 {
    #[field(name = "foo")]
    single: usize,
    #[field(name = "foo")]
    other: usize,
}

#[derive(FromForm)]
struct Renamed2 {
    #[field(name = uncased("HELLO_THERE"))]
    single: usize,
    hello_there: usize,
}

#[derive(FromForm)]
struct Renamed3 {
    #[field(name = "hello_there")]
    single: usize,
    hello_there: usize,
}

fn main() { }

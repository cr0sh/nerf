use nerf_macros::tag;

#[test]
fn test_tag() {
    trait Foo {
        type Foo;
    }
    struct FooValue;

    #[tag(Foo = FooValue)]
    struct Bar;

    fn check_foo<T: Foo>(_: T) -> T::Foo {
        unreachable!();
    }

    if false {
        check_foo(Bar);
    }
}

use super::*;

fn expansion(source: &str) -> syn::Result<Tokens> {
    expand_for_crate(&syn::parse_str(source)?, &quote!(::nyx_ssz))
}

#[test]
fn should_reject_invalid_declarations_when_deriving_containers_and_unions() {
    for source in [
        "struct Empty {}",
        "struct Tuple(u8);",
        "struct Unit;",
        "union Raw { a: u8 }",
        "#[ssz(unknown)] struct A { a: u8 }",
        "#[ssz(progressive, progressive)] struct A { a: u8 }",
        "#[ssz(rename=\"a\")] struct A { a: u8 }",
        "#[ssz(position=0)] struct A { a: u8 }",
        "#[ssz(progressive)] struct A { a: u8 }",
        "#[ssz(progressive)] struct A { #[ssz(position=256)] a: u8 }",
        "#[ssz(progressive)] struct A { #[ssz(position=0)] a:u8, #[ssz(position=0)] b:u8 }",
        "struct A { #[ssz(rename=\"x\")] a:u8, #[ssz(rename=\"x\")] b:u8 }",
        "struct A { #[ssz(position=0)] a:u8 }",
        "struct A { #[ssz(selector=1)] a:u8 }",
        "struct A { #[ssz(progressive)] a:u8 }",
        "enum A {}",
        "enum A { B(u8) }",
        "#[ssz(progressive)] enum A { #[ssz(selector=1)] B(u8) }",
        "enum A { #[ssz(selector=0)] B(u8) }",
        "enum A { #[ssz(selector=128)] B(u8) }",
        "enum A { #[ssz(selector=1)] B(u8), #[ssz(selector=1)] C(u8) }",
        "enum A { #[ssz(selector=1)] B }",
        "enum A { #[ssz(selector=1)] B(u8,u8) }",
        "enum A { #[ssz(selector=1)] B { x:u8 } }",
        "enum A { #[ssz(selector=1)] B(u8)=1 }",
        "enum A { #[ssz(selector=1,rename=\"x\")] B(u8) }",
        "enum A { #[ssz(selector=1)] B(#[ssz(rename=\"x\")] u8) }",
    ] {
        assert!(expansion(source).is_err(), "accepted {source}");
    }
}

#[test]
fn should_generate_complete_rust_items_when_generic_or_renamed_types_are_valid() {
    for source in [
        "struct A { a:u8 }", "struct A<T> where T: Copy { a:T }",
        "#[ssz(progressive)] struct A { #[ssz(position=255)] z:u8, #[ssz(position=0,rename=\"日本語\\0\")] r#type:bool }",
        "enum A<T> { #[ssz(selector=127)] B(T), #[ssz(selector=1)] C(T) }",
    ] {
        let tokens = expansion(source).unwrap();
        assert!(
            syn::parse2::<syn::File>(tokens.clone()).is_ok(),
            "invalid output for {source}"
        );
        if source.starts_with("struct A") {
            let generated = tokens.to_string();
            assert!(generated.contains("encode_fixed_sequence"));
            assert!(generated.contains("decode_fixed_sequence"));
        }
    }
}

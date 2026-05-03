use slangmake::{Permutation, PermutationDefineKind, PermutationDefines};

#[test]
fn permutation_key_is_alphabetical() {
    let mut p = Permutation::new().unwrap();
    p.add_constant("ZED", "1").unwrap();
    p.add_constant("ALPHA", "2").unwrap();
    p.add_constant("MIKE", "3").unwrap();
    let key = p.key().unwrap().to_string();
    let alpha = key.find("ALPHA").expect("ALPHA in key");
    let mike = key.find("MIKE").expect("MIKE in key");
    let zed = key.find("ZED").expect("ZED in key");
    assert!(alpha < mike && mike < zed, "key not alphabetical: {key}");
}

#[test]
fn permutation_iteration_returns_all_constants() {
    let mut p = Permutation::new().unwrap();
    p.add_constant("FOO", "1").unwrap();
    p.add_constant("BAR", "2").unwrap();
    let pairs: Vec<(String, String)> = p
        .constants()
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect();
    assert_eq!(pairs.len(), 2);
    assert!(pairs.contains(&("FOO".into(), "1".into())));
    assert!(pairs.contains(&("BAR".into(), "2".into())));
}

#[test]
fn permutation_type_args_separate_from_constants() {
    let mut p = Permutation::new().unwrap();
    p.add_constant("USE_FOO", "1").unwrap();
    p.add_type_arg("T", "float").unwrap();
    assert_eq!(p.constants().count(), 1);
    assert_eq!(p.type_args().count(), 1);
    let (name, value) = p.type_args().next().unwrap();
    assert_eq!(name, "T");
    assert_eq!(value, "float");
}

#[test]
fn permutation_defines_push_and_iter() {
    let mut defs = PermutationDefines::new().unwrap();
    defs.push("ENABLE_X", PermutationDefineKind::Constant, &["0", "1"])
        .unwrap();
    defs.push(
        "MATERIAL_T",
        PermutationDefineKind::Type,
        &["Lambert", "Phong"],
    )
    .unwrap();
    assert_eq!(defs.len(), 2);

    let entries: Vec<_> = defs
        .iter()
        .map(|(n, k, v)| {
            (
                n.to_string(),
                k,
                v.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(entries[0].0, "ENABLE_X");
    assert_eq!(entries[0].1, PermutationDefineKind::Constant);
    assert_eq!(entries[0].2, vec!["0", "1"]);
    assert_eq!(entries[1].0, "MATERIAL_T");
    assert_eq!(entries[1].1, PermutationDefineKind::Type);
}

#[cfg(feature = "runtime")]
#[test]
fn permutation_defines_parse_source_then_expand() {
    // The upstream parser recognises `// [permutation] NAME={VALUES}` magic
    // comments. Two single-value axes produce a 1x1 = 1 permutation.
    let src = r#"
// [permutation] ENABLE_FOG={0}
// [permutation] LIGHTS={1}
[shader("compute")] [numthreads(1,1,1)] void main(uint3 t : SV_DispatchThreadID) {}
"#;
    let defs = PermutationDefines::parse_source(src).unwrap();
    assert_eq!(defs.len(), 2);

    let list = defs.expand().unwrap();
    assert_eq!(list.len(), 1);
    let only = list.get(0).unwrap();
    let key = only.key().unwrap().to_string();
    assert!(key.contains("ENABLE_FOG"));
    assert!(key.contains("LIGHTS"));
}

#[cfg(feature = "runtime")]
#[test]
fn permutation_defines_cartesian_product_size() {
    let mut defs = PermutationDefines::new().unwrap();
    defs.push("A", PermutationDefineKind::Constant, &["0", "1"])
        .unwrap();
    defs.push("B", PermutationDefineKind::Constant, &["x", "y", "z"])
        .unwrap();
    let list = defs.expand().unwrap();
    assert_eq!(list.len(), 6);
}

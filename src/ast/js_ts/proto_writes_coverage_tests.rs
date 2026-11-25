use super::{js, read};

#[test]
fn accessor_shadow_proof_rejects_unknown_and_large_literals() {
    for literal in [
        "{...other}",
        "{[dynamic]: 1}",
        "{other: 1}",
        "{a:0,b:0,c:0,d:0,e:0,f:0,g:0,h:0,i:0,j:0,k:0,l:0,m:0,n:0,o:0,p:0,q:0,r:0,s:0,t:0,u:0,v:0,w:0,x:0,y:0,z:0,a1:0,b1:0,c1:0,d1:0,e1:0,f1:0,g1:0}",
    ] {
        let file = js(&format!("const evidence={{get key() {{return 1}}}};\nfunction f(other,dynamic) {{const obj={literal};\n return obj.key; }}"));
        let value = read(&file, 3, "obj.key");
        assert!(!value.resolution_complete, "{literal}: {value:?}");
        assert_eq!(value.incomplete_reason, Some("member_accessor"), "{literal}");
    }
    let control = js("const evidence={get key(){return 1}};\nconst obj={'key':1};\nobj.key;");
    assert!(read(&control, 3, "obj.key").resolution_complete);
}

#[test]
fn listing_refuses_history_when_capacity_is_exhausted() {
    let file = js("function f(o){ helper(o.child);\n return o.child.key; }");
    let ordinary = read(&file, 2, "o.child.key");
    assert_eq!(ordinary.incomplete_reason, Some("member_call"));
    assert!(!ordinary.reaching_versions.is_empty());
    let mut capacity = 0;
    let listed = file.js_ts_listed_name_reads(2, &mut capacity);
    let value = listed
        .iter()
        .find(|value| value.path == "o.child.key")
        .unwrap();
    assert!(value.candidate_history_refused);
    assert!(value.reaching_versions.is_empty());
    let mut capacity = 100;
    let listed = file.js_ts_listed_name_reads(2, &mut capacity);
    let value = listed
        .iter()
        .find(|value| value.path == "o.child.key")
        .unwrap();
    assert!(!value.candidate_history_refused);
    assert_eq!(value.reaching_versions, ordinary.reaching_versions);
}

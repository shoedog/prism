// Every control is a repository: global ambient fences must not cross fixtures.
mod js_paths;
mod js_paths_first_pass;
mod js_paths_snapshot;
mod js_paths_syntax;
fn main() {
    let root=std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let requests:Vec<(String,String)>=serde_json::from_slice(&std::fs::read(std::env::args().nth(2).unwrap()).unwrap()).unwrap();
    let mut grouped=std::collections::BTreeMap::<String,Vec<(String,String,String)>>::new();
    for (file,spec) in requests {
        let (case,local)=file.split_once('/').unwrap();
        grouped.entry(case.into()).or_default().push((file.clone(),local.into(),spec));
    }
    let mut out=Vec::new();
    for (case,requests) in grouped {
        let snapshot=js_paths_snapshot::JsPathsSnapshot::capture(&root.join(&case));
        let indexed=snapshot.entries.iter().filter(|(p,k)|**k==0&&[".ts",".tsx",".js",".jsx"].iter().any(|e|p.ends_with(e))).map(|(p,_)|p.clone()).collect();
        let mut resolver=js_paths::Resolver::new(&snapshot);
        for (file,local,spec) in requests {
            let target=resolver.resolve(&local,&spec,&indexed).map(|p|format!("{case}/{p}"));
            out.push((file,spec,target));
        }
    }
    println!("{}",serde_json::to_string(&out).unwrap());
}

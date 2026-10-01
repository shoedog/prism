// Compile with copies of the prototype's actual three resolver modules beside this file.
mod js_paths;
mod js_paths_snapshot;
mod js_paths_syntax;
fn main() {
    let root=std::env::args().nth(1).unwrap();
    let requests=std::env::args().nth(2).unwrap();
    let snapshot=js_paths_snapshot::JsPathsSnapshot::capture(std::path::Path::new(&root));
    let indexed=snapshot.entries.iter().filter(|(p,k)|**k==0&&[".ts",".tsx",".js",".jsx"].iter().any(|e|p.ends_with(e))).map(|(p,_)|p.clone()).collect();
    let requests:Vec<(String,String)>=serde_json::from_slice(&std::fs::read(requests).unwrap()).unwrap();
    let mut resolver=js_paths::Resolver::new(&snapshot);
    let out:Vec<_>=requests.iter().map(|(f,s)|(f,s,resolver.resolve(f,s,&indexed))).collect();
    println!("{}",serde_json::to_string(&out).unwrap());
}

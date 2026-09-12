fn main() {
    println!("cargo:rerun-if-env-changed=MODE");
    if std::env::var("MODE").as_deref() == Ok("on") {
        println!("cargo:rustc-cfg=mode_on");
    }
}

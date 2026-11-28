use super::{js, KeyRole};

#[test]
fn same_line_inventory_distinguishes_stable_reads_rebinding_and_owners() {
    let source = concat!(
        "function update(root, v, key, next, field) {\n",
        "  if (key) root[key][field] = other[key]; root[key][field] = key;\n",
        "  root[key][field] = key; key = next; sink(key);\n",
        "  [key].map(key => read(key)); read(key); root[key][field] = missing;\n",
        "  read(key); read(unknown);\n",
        "}\n"
    );
    let file = js(source);
    let reads = |line| {
        file.js_ts_name_reads(line)
            .into_iter()
            .filter(|read| read.base == "key")
            .collect::<Vec<_>>()
    };
    let stable = reads(2);
    assert_eq!(stable.len(), 5);
    assert!(stable.iter().all(|read| {
        read.binding == stable[0].binding
            && read.owner == stable[0].owner
            && read.reaching_versions == stable[0].reaching_versions
            && read.resolution_complete
    }));
    let facts = file.js_ts_proto_writes();
    assert_eq!(facts.iter().filter(|write| write.line == 2).count(), 2);
    assert!(facts.iter().filter(|write| write.line == 2).all(|write| {
        let selector = write
            .key_operands
            .iter()
            .find(|operand| operand.role == KeyRole::Selector)
            .unwrap();
        selector.binding == stable[0].binding
            && selector.reaching_versions == stable[0].reaching_versions
            && selector.start_byte > stable[0].span.start_byte
    }));
    let rebound = reads(3);
    assert_eq!(rebound.len(), 3);
    assert_eq!(rebound[0].reaching_versions, rebound[1].reaching_versions);
    assert_eq!(rebound[0].binding, rebound[2].binding);
    assert_ne!(rebound[0].reaching_versions, rebound[2].reaching_versions);
    assert_eq!(
        rebound[2].reaching_versions[0].start_byte,
        source.find("key = next").unwrap()
    );

    let sibling = reads(4);
    assert_eq!(sibling.len(), 4);
    assert_ne!(sibling[1].owner, sibling[2].owner);
    assert_ne!(sibling[1].binding, sibling[2].binding);
    assert_eq!(sibling[2].binding, sibling[3].binding);
    assert!(file
        .js_ts_name_reads(4)
        .iter()
        .any(|read| read.path == "missing" && read.binding.is_none() && !read.resolution_complete));
    assert_eq!(reads(5).len(), 1);
    assert!(file
        .js_ts_name_reads(5)
        .iter()
        .any(|read| read.path == "unknown" && !read.resolution_complete));
    assert!(file.js_ts_name_reads(6).is_empty());
    assert!(file.js_ts_name_reads(0).is_empty());
}

#[test]
fn shadowed_or_mutated_reads_remain_in_the_inventory() {
    let file = js(concat!(
        "function run(part, one, two, cond) { const opts={key:0};\n",
        "  const before=opts.key; opts[part] = two; const after=opts.key;\n",
        "  if (cond) part = one; read(part);\n",
        "  { read(part); let part = two; read(part); }\n",
        "}\n"
    ));
    let reads = file.js_ts_name_reads(2);
    let member: Vec<_> = reads
        .iter()
        .filter(|read| read.path == "opts.key")
        .collect();
    assert_eq!(member.len(), 2);
    assert!(member[0].resolution_complete);
    assert!(!member[1].resolution_complete);
    let join = file
        .js_ts_name_reads(3)
        .into_iter()
        .find(|read| read.base == "part")
        .unwrap();
    assert_eq!(join.reaching_versions.len(), 2);
    let local: Vec<_> = file
        .js_ts_name_reads(4)
        .into_iter()
        .filter(|read| read.base == "part")
        .collect();
    assert_eq!(local.len(), 2);
    assert_ne!(local[0].binding, join.binding);
    assert_eq!(local[0].binding, local[1].binding);
    assert!(!local[0].resolution_complete);
    assert!(local[1].resolution_complete);
}

#[test]
fn same_line_unresolved_competitor_retains_its_program_owner() {
    let source = "function f(o, key, p) { o[key][p] = key; } read(key);";
    let file = js(source);
    let reads: Vec<_> = file
        .js_ts_name_reads(1)
        .into_iter()
        .filter(|read| read.base == "key")
        .collect();
    assert_eq!(reads.len(), 3);
    assert_eq!(reads[0].binding, reads[1].binding);
    assert_eq!(reads[2].owner.end_byte, source.len());
    assert!(
        reads[2].binding.is_none()
            && reads[2].reaching_versions.is_empty()
            && !reads[2].resolution_complete
    );
}

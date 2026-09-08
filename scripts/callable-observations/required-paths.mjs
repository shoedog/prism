// Pinned TS 5.9.3 Program-cache proof, not a new filesystem resolver.
// Missing candidate probes are not required-input failures. A path directive is.
export function hasUnprovenRequiredPath(program,toId,read) {
  const sources=new Set(program.getSourceFiles()),includeReasons=program.getFileIncludeReasons();
  let unproven=false;
  for(const sf of sources) {
    // Package-ID redirects inherit another file's AST. Census the captured
    // original, while using the configured Program's canonical owner path.
    const source=sf.redirectInfo?.unredirected??sf;
    if(!source.referencedFiles.length)continue;
    const sourceId=toId(source.fileName);
    if(!sourceId || sourceId!==toId(sf.fileName) || read(source.fileName)!==source.text)throw Error('unsupported_input');
    for(const [index,ref] of source.referencedFiles.entries()) {
      // This is a read-only lookup using the compiler's supported extensions,
      // extensionless ordering, canonical paths and already-built source cache.
      const target=program.getSourceFileFromReference(source,ref);
      const targetId=target&&toId(target.fileName);
      const included=target && includeReasons.get(target.path)?.some(r=>
        r.kind===4 /* TS FileIncludeKind.ReferenceFile */ && r.file===sf.path && r.index===index);
      // Incidental membership, disabled traversal and redirect-inherited reasons
      // cannot discharge this original occurrence. Self-reference is not valid.
      if(!target || !sources.has(target) || !targetId || targetId===sourceId || !included)unproven=true;
    }
  }
  return unproven;
}

import {readdirSync,lstatSync,realpathSync,openSync,readSync,closeSync,fstatSync,constants} from "node:fs";
import {createHash} from "node:crypto";
import path from "node:path";
import {relative,hash,canonical} from "./schema.mjs";
const fail=reason=>{throw Error(reason);};
// One bounded chunk while hashing; only compiler-requested reads retain content.
function content(absolute,budget,retain=false) {
  const fd=openSync(absolute,constants.O_RDONLY|constants.O_NOFOLLOW);
  try {
    const before=fstatSync(fd);
    if(!before.isFile())fail("unsupported_input");
    if(before.size>budget)fail("budget_exceeded");
    const digest=createHash("sha256"),chunk=Buffer.alloc(65536),chunks=[];let size=0;
    for(;;) {
      const n=readSync(fd,chunk,0,Math.min(chunk.length,budget-size+1),null);if(!n)break;
      size+=n;if(size>budget)fail("budget_exceeded");
      digest.update(chunk.subarray(0,n));if(retain)chunks.push(Buffer.from(chunk.subarray(0,n)));
    }
    const after=fstatSync(fd);
    if(size!==before.size || before.size!==after.size || before.mtimeMs!==after.mtimeMs || before.ctimeMs!==after.ctimeMs)fail("unstable_snapshot");
    return {sha256:digest.digest("hex"),size,bytes:retain?Buffer.concat(chunks,size):undefined};
  } finally {closeSync(fd);}
}
export function snapshot(options) {
  const files=new Map(),directories=new Map();let bytes=0;
  const roots={project:realpathSync(options.root),compiler:realpathSync(path.dirname(options.compiler))};
  if(roots.project===path.parse(roots.project).root || roots.project===roots.compiler) fail("unsupported_input");
  function walk(absolute,id,depth) {
    if(depth>options.limits.depth || files.size+directories.size>=options.limits.files) fail("budget_exceeded");
    const stat=lstatSync(absolute);
    if(stat.isSymbolicLink()) fail("unsupported_input");
    if(stat.isDirectory()) {
      const entries=readdirSync(absolute).sort();directories.set(id,entries);
      for(const name of entries) {
        if(name.toLowerCase()===".git") {directories.set(id+"/"+name,[]);continue;}
        if(!relative(name) || name.includes("/")) fail("unsupported_input");
        walk(path.join(absolute,name),id+"/"+name,depth+1);
      }
    } else if(stat.isFile()) {
      if(bytes+stat.size>options.limits.bytes) fail("budget_exceeded");
      const record=content(absolute,options.limits.bytes-bytes);bytes+=record.size;
      files.set(id,{id,sha256:record.sha256,size:record.size});
    } else fail("unsupported_input");
  }
  for(const [id,absolute] of Object.entries(roots)) walk(absolute,id,0);
  const manifest=[...files.values()].sort((a,b)=>a.id<b.id?-1:1);
  const dirs=[...directories.keys()].sort();
  const read=id=>{
    const expected=files.get(id);if(!expected)return undefined;
    if(expected.size>options.limits.read_bytes)fail("budget_exceeded");
    const [root,...parts]=id.split("/");
    try {
      const actual=content(path.join(roots[root],...parts),expected.size,true);
      if(actual.sha256!==expected.sha256 || actual.size!==expected.size)fail("unstable_snapshot");
      return actual.bytes;
    } catch {fail("unstable_snapshot");}
  };
  return {files,directories,roots,manifest,dirs,digest:hash(canonical({files:manifest,directories:dirs})),read};
}

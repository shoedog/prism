// Separate detached envelope; never a serialized authority object.
import {readFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
import path from "node:path";
import {build} from "./worker.mjs";
import {settings} from "./index.mjs";
import {parsePacket} from "./schema.mjs";
import {directOwnerFacts} from "./direct-owner.mjs";

export function derive(options) {
  let facts;
  const packet=build(settings(options),context=>{facts=directOwnerFacts(context);});
  parsePacket(JSON.stringify(packet));
  return {schema:"prism.detached-owner/1",authorizes_runtime_edge:false,packet,...facts};
}
if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  try {console.log(JSON.stringify(derive(JSON.parse(readFileSync(0,"utf8")))));}
  catch {process.stderr.write("detached_owner_acquisition_failed\n");process.exitCode=1;}
}

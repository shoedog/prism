import {classifyBoundary} from './search-provenance.mjs';

// Compiler-generated containing-file addresses are not canonical source files.
// Keep selected config spelling: no filesystem, manifest, case folding or links.
export function projectSyntheticAddress(file) {
  if(typeof file!=='string')throw Error('unsupported_input');
  const value=classifyBoundary(file);
  if(value.kind!=='in_root'||!value.id.startsWith('project/'))throw Error('unsupported_input');
  return value.id;
}

import { exec } from 'child_process';
declare function acquire(): string;
import { load as decode } from 'js-yaml';
function entry() {
  const t = acquire(); // @SRC
  return decode(t, { json: true }); // @SINK
}
entry();

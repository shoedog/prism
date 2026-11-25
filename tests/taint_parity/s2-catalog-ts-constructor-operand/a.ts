import { exec } from 'child_process';
declare function acquire(): string;
function entry() {
  const t = acquire(); // @SRC
  const fixed = 'fixed';
  observe(t); new Function(fixed); // @SINK
}
entry();

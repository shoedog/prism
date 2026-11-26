import { exec } from 'child_process';
function entry() {
  const t=acquire(); // @SRC
  const x=escapeHtml(t);
  exec(x); // @SINK
}

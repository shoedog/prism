import { exec } from 'child_process';
function entry() {
  const t = acquire(); // @SRC
  const clean = escapeHtml(t); const out = "fixed";
  exec(out); // @SINK
}

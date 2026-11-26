import { exec } from 'child_process';
function entry(flag) {
  const t = acquire(); // @SRC
  const escaped = escapeHtml(t);
  const cmd = flag ? escaped : t;
  exec(cmd); // @SINK
}
entry(Boolean(choice()));

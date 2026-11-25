import { exec } from 'child_process';
function entry(flag: boolean) {
  const t = acquire(); // @SRC
  const u = acquire();
  const first = escapeHtml(t);
  const second = DOMPurify.sanitize(u);
  const value = flag ? first : second;
  exec(value); // @SINK
}

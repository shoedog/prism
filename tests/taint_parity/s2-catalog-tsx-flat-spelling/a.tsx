import { exec } from 'child_process';
declare function acquire(): string;
function entry() {
  const t = acquire(); // @SRC
  const dangerouslySetInnerHTML = t; observe(dangerouslySetInnerHTML); // @SINK
}
entry();

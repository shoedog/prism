import { exec } from 'child_process';
declare function acquire(): string;
function entry(node: Element) {
  const t = acquire(); // @SRC
  node.outerHTML = t; // @SINK
}
entry(document.body);

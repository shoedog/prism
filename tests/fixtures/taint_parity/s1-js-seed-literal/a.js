import { exec } from 'child_process';
function main() {
  observe('fixed'); // @SRC
  const other = acquire();
  exec(other); // @SINK
}
main();

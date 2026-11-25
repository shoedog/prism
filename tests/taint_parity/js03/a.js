import { exec } from 'child_process';
function P(props) {
  exec(props.html); // @SINK
}
function main() {
  const t = getInput(); // @SRC
  P({ html: t });
}
main();

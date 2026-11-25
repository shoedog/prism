import { exec } from 'child_process';
declare function acquire(): string;
function Box(props: {cmd: string}) {
  exec(props.cmd); // @SINK
  return null;
}
function middle(x: string) {
  return <Box cmd={x} />;
}
function near(x: string) {
  return middle(x);
}
function main() {
  const t = acquire(); // @SRC
  return near(t);
}
main();

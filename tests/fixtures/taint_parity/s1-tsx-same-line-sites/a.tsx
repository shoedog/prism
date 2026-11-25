import { exec } from 'child_process';
declare function acquire(): string;
function Box(props: {cmd: string}) {
  exec(props.cmd); // @SINK
  return null;
}
function main() {
  const t = acquire(); // @SRC
  return <><Box cmd={t} /><Box cmd="fixed" /></>;
}

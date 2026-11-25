import { exec } from 'child_process';
declare function acquire(): string;
function Box(props: {cmd: string}) {
  const run = () => {
    exec(props.cmd); // @SINK
  };
  run();
  return null;
}
function main() {
  const t = acquire(); // @SRC
  return <Box cmd={t} />;
}
main();

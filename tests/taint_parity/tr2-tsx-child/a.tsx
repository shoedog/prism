import { exec } from 'child_process';
function Box(props: {children: string}) {
  exec(props.children); // @SINK
  return null;
}
function main() {
  const t = acquire(); // @SRC
  return <Box>{t}</Box>;
}

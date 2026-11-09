function Comp(props: { a: number }) {
  let n = props.a;
  n = 2;
  n = 3;
  return <div>{n}</div>;
}

const Arrow = (props: { b: number }) => {
  let m = props.b;
  m = 4;
  return <span>{m}</span>;
};

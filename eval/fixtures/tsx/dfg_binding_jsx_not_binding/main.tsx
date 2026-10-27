function f(): JSX.Element {
  let x = source();
  const view = <C x={x}><x:item>{x}</x:item></C>;
  sink(x);
  return view;
}

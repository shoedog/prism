function keep<F>(fn: F): F { return fn; }
const View = keep((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
function entry() {
  return [0].map(() => {
    const t = acquire(); // @SRC
    return <View value={t}/>;
  });
}

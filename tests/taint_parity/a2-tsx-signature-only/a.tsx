function View(p: {value: string}) {
  observe(p.value); // @SINK
  return null;
}
const render = (t: string) => <View value={t}/>;
function entry() {
  const t = acquire(); // @SRC
  return render(t);
}

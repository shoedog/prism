function View(p: {value: string}) {
  observe(p.value); // @SINK
  return null;
}
function entry() {
  return [0].map(function render() {
    const t = acquire(); // @SRC
    return <View value={t}/>;
  });
}

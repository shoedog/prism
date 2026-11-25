function f(inp) {
  const t = inp.value; // @SRC
  return <div dangerouslySetInnerHTML={t} />; // @SINK
}

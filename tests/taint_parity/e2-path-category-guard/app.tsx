function f(inp) {
  const t = inp.value; // @SRC
  const v = DOMPurify.sanitize(t);
  return <div dangerouslySetInnerHTML={{__html: v}} />; // @SINK
}

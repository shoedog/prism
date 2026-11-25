function entry(flag: boolean, second: boolean) {
  const t = acquire(); // @SRC
  const u = acquire();
  const clean = DOMPurify.sanitize(t);
  const value = second ? u : (flag ? clean : t);
  return <div dangerouslySetInnerHTML={{__html: value}}/>; // @SINK
}

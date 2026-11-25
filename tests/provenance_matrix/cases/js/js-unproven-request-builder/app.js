function build() { return { body: "fixed" }; }

function read() {
  const req = build();
  const payload = req.body;
  return payload;
}

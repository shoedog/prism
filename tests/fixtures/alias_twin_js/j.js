function t(o, q) {
  let u = o;            // EXPECT u:binding-write:S-01
  u = q;                // EXPECT u:binding-write:S-01
  u += 1;               // EXPECT u:binding-write:S-01
  u.f = 1;              // EXPECT u.f:storage-write:S-04
  u[0] = 2;             // EXPECT u[]:storage-write:S-04
  const { a, b: c } = o;    // EXPECT a:binding-write:S-02 c:binding-write:S-02
  ({ k: u.y } = o);     // EXPECT u.y:storage-write:S-04
  [u] = [o];            // EXPECT u:binding-write:S-03
  u ||= o;              // EXPECT u:binding-write:S-01
  var w;                // EXPECT w:binding-write:S-05
  for (u of q) {}       // EXPECT u:binding-write:S-01
  for (const [k, ...r] of q) {}   // EXPECT k:binding-write:S-02 r:binding-write:S-02
  try { f(); } catch (e) {}
}
function adversarial(o, xs) {
  let u = o;            // EXPECT u:binding-write:S-01
  for (u.x of xs) {}
  u ||= o;              // EXPECT u:binding-write:S-01
}
function anch(o) {
  const {
    a,
    b: { c },
  } = o;
}

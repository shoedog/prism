export class C {
  static escaped_target() { return 0; }
}
const Alias = C;
function replacement() { return 1; }
Alias.escaped_target = replacement;

function visit(item) {
  use(item);
}
export function main() {
  const items = [1];
items.forEach(visit);
}

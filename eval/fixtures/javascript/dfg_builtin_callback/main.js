function visit(item) {
  use(item);
}
function main() {
  const items = [1];
items.forEach(visit);
}

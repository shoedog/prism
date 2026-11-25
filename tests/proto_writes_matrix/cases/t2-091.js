function put(root, input, field, value) {
  for (const key of Object.getOwnPropertySymbols(input)) {
    if (root[key] == null) continue;
    root[key][field] = value;
  }
  return root;
}

function putSymbolLabels(labels, field, value) {
  const symbols = new Map([
    ["display", Symbol("display")],
    ["profile", Symbol("profile")]
  ]);
  const root = Object.fromEntries(Array.from(symbols.values(), key => [key, {}]));
  const input = Object.fromEntries(
    labels.map(label => [symbols.get(label) ?? Symbol(label), true])
  );
  return put(root, input, field, value);
}

module.exports = putSymbolLabels;

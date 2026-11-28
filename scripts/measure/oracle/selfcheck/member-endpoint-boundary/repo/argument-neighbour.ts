export {};
function pick(p: number) { return p; }
let sink = 0;
const source = { pickKey: 2 };
const value = 1;
pick(value); ({ pickKey: sink } = source);
pick(value);

function identity(
  formal: number) { return formal; }
const actual = 0;
identity(actual);

function pair(left: number, right: number) { return left + right; }
const alpha = 1;
const beta = 2;
pair(alpha, beta);

function optional(maybe?: number) { return maybe; }
const supplied = 3;
optional(supplied);

function defaulted(count: number = 0) { return count; }
const explicit = 4;
defaulted(explicit);

function echo(café: number) { return café; }
function route(signal: number) {
  echo(
    /* 😀 */ signal
  );
}

function chain(seed: number) {
  const first = seed;
  const second = first;
  return second;
}
function branchy(flag: boolean, value: number) {
  const picked = flag ? value : 0;
  return picked;
}
function looped(items: number[]) {
  let total = 0;
  for (const item of items) { total = total + item; }
  return total;
}
function pairUp(alpha2: number) {
  const copy2 = alpha2;
  const twice = copy2 + copy2;
  return twice;
}

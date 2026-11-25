# Prototype-write sink rows

## sk01

expect: fires

Both marked writes fire.

```js
function set(obj, path, value) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    o = o[keys[i]] = o[keys[i]] || {}; // SINK
  }
  o[keys[keys.length - 1]] = value; // SINK
}

function update(obj, input) {
  const path = input.path; // SEED
  set(obj, path, 1);
}
```

## sk02

expect: silent

```js
function set(
  obj,
  path,
  value
) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    o = o[keys[i]] = o[keys[i]] || {};
  }
  o[keys[keys.length - 1]] = value; // SINK
}

function update(obj, input) {
  const value = input.value; // SEED
  set(obj, "a.b", value);
}
```

## sk03a

expect: silent

```js
function update(input, k1, k2, v) {
  const obj = input.receiver; // SEED
  obj[k1][k2] = v; // SINK
}
```

## sk03b

expect: fires

Known receiver-only false positive ([#854](https://github.com/shoedog/prism/issues/854)).

```js
// Known receiver-only false positive from #854.

function set(obj, path, value) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    o = o[keys[i]] = o[keys[i]] || {};
  }
  o[keys[keys.length - 1]] = value; // SINK
}

function update(input) {
  const obj = input.receiver; // SEED
  set(obj, "a.b", 1);
}
```

## sk03c

expect: silent

```js
function update(input, k1, k2, v) {
  const obj = input.receiver; // SEED
  const u = obj[k1];
  u[k2] = v; // SINK
}
```

## sk04

expect: silent

```js
function set(obj, path, value) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    const k = keys[i];
    if (k === "__proto__" || k === "constructor" || k === "prototype") return;
    o = o[k] = o[k] || {};
  }
  o[keys[keys.length - 1]] = value; // SINK
}

function update(obj, input) {
  const path = input.path; // SEED
  set(obj, path, 1);
}
```

## sk05

expect: fires

description contains: partial key guard

```js
function set(obj, path, value) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    const k = keys[i];
    if (k === "__proto__") return;
    o = o[k] = o[k] || {};
  }
  o[keys[keys.length - 1]] = value; // SINK
}

function update(obj, input) {
  const path = input.path; // SEED
  set(obj, path, 1);
}
```

## sk06

expect: silent

```js
function cacheValue(cache, input, v) {
  const id = input.id; // SEED
  cache[id] = v; // SINK
}
```

## sk07

expect: fires

```js
function set(obj, path, value) {
  const keys = path.split(".");
  let o = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    o = o[keys[i]] = o[keys[i]] || {};
  }
  o[keys[keys.length - 1]] = value; // SINK
}

function forward(obj, path) {
  set(obj, path, 1);
}

function update(obj, input) {
  const path = input.path; // SEED
  forward(obj, path);
}
```

## sk08

expect: fires

```js
function update(o, opts, name, input, v) {
  opts.key = input.key; // SEED
  o[opts.key][name] = v; // SINK
}
```

## sk09

expect: fires

```js
function merge(target, source) {
  for (const key in source) {
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      merge(target[key], source[key]);
    } else {
      target[key] = source[key]; // SINK
    }
  }
}

function update(target, input) {
  const source = input.record; // SEED
  merge(target, source);
}
```

## sk10

expect: silent

```js
function merge(target, source) {
  for (const key in source) {
    if (key === "__proto__" || key === "constructor") continue;
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      merge(target[key], source[key]);
    } else {
      target[key] = source[key]; // SINK
    }
  }
}

function update(target, input) {
  const source = input.record; // SEED
  merge(target, source);
}
```

## sk11

expect: silent

Accepted loss for callback parameter binding ([#434](https://github.com/shoedog/prism/issues/434)).

```js
function merge(target, source) {
  Object.keys(source).forEach(key => {
    if (typeof source[key] === "object") {
      if (!target[key]) target[key] = {};
      merge(target[key], source[key]);
    } else {
      target[key] = source[key]; // SINK
    }
  });
}

function update(target, input) {
  const source = input.record; // SEED
  merge(target, source);
}
```

## sk12

expect: silent

```js
function put(store, kind, id, v) {
  store[kind][id] = v; // SINK
}

function update(store, input) {
  const id = input.id; // SEED
  put(store, "items", id, 1);
}
```

## sk13

expect: silent

```js
function setIn(o, path, last, v) {
  for (const k of path) {
    o = o[k] = o[k] || {};
  }
  o[last] = v; // SINK
}

function update(o, input) {
  const last = input.last; // SEED
  setIn(o, ["items"], last, 1);
}
```

## sk14

expect: fires

```js
const express = require("express");
const app = express();
const store = {};

app.get("/items/:kind/:id", function (req, res) {
  const id = req.params.id; // SEED
  if (!id) return res.sendStatus(400);
  store[req.params.kind][req.params.id] = 1; // SINK
  return res.sendStatus(204);
});
```

## sk15

expect: fires

finding count: 1
description suffix: (the line also holds another taint sink)

```js
function update(out, input, k2) {
  const k1 = input.key, code = input.code; // SEED
  out[k1][k2] = eval(code); // SINK
}
```

## sk16a

expect: fires

```js
function update(a, input, p, q, v) {
  const k = input.key; // SEED
  if (typeof k === "string" && k !== "__proto__" && k !== "constructor" && k !== "prototype") { a[k][p] = v; } a[k][q] = v; // SINK
}
```

## sk16b

expect: silent

```js
function update(a, input, m, p, q, v) {
  const k = input.key; // SEED
  if (typeof k === "string" && k !== "__proto__" && k !== "constructor" && k !== "prototype") { a[k][p] = v; } a[m][q] = v; // SINK
}
```

## sk17

expect: fires

severity: warning

```js
function update(out, input, k2, v, knex) {
  const k1 = input.key; // SEED
  out[k1][k2] = v; knex("items").whereRaw("id = ?", [k1]); // SINK
}
```

## skx-plain-function-yes

expect: fires

```js
function put(a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1][k2] = s[k1][k2]; // SINK
}
```

## skx-plain-function-no

expect: fires

```js
function put(a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1][k2] = v; // SINK
}
```

## skx-plain-method-yes

expect: fires

```js
const writer = {
  put(a, input, k2, s) {
    const k1 = input.key; // SEED
    a[k1][k2] = s[k1][k2]; // SINK
  }
};
```

## skx-plain-method-no

expect: fires

```js
const writer = {
  put(a, input, k2, v) {
    const k1 = input.key; // SEED
    a[k1][k2] = v; // SINK
  }
};
```

## skx-plain-arrow-yes

expect: fires

```js
const put = (a, input, k2, s) => {
  const k1 = input.key; // SEED
  a[k1][k2] = s[k1][k2]; // SINK
};
```

## skx-plain-arrow-no

expect: fires

```js
const put = (a, input, k2, v) => {
  const k1 = input.key; // SEED
  a[k1][k2] = v; // SINK
};
```

## skx-plain-property-yes

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1][k2] = s[k1][k2]; // SINK
};
```

## skx-plain-property-no

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1][k2] = v; // SINK
};
```

## skx-if-function-yes

expect: fires

```js
function put(a, input, k2, s) {
  const k1 = input.key; // SEED
  if (a[k1]) {
    a[k1][k2] = s[k1][k2]; // SINK
  }
}
```

## skx-if-function-no

expect: fires

```js
function put(a, input, k2, v) {
  const k1 = input.key; // SEED
  if (a[k1]) {
    a[k1][k2] = v; // SINK
  }
}
```

## skx-if-method-yes

expect: fires

```js
const writer = {
  put(a, input, k2, s) {
    const k1 = input.key; // SEED
    if (a[k1]) {
      a[k1][k2] = s[k1][k2]; // SINK
    }
  }
};
```

## skx-if-method-no

expect: fires

```js
const writer = {
  put(a, input, k2, v) {
    const k1 = input.key; // SEED
    if (a[k1]) {
      a[k1][k2] = v; // SINK
    }
  }
};
```

## skx-if-arrow-yes

expect: fires

```js
const put = (a, input, k2, s) => {
  const k1 = input.key; // SEED
  if (a[k1]) {
    a[k1][k2] = s[k1][k2]; // SINK
  }
};
```

## skx-if-arrow-no

expect: fires

```js
const put = (a, input, k2, v) => {
  const k1 = input.key; // SEED
  if (a[k1]) {
    a[k1][k2] = v; // SINK
  }
};
```

## skx-if-property-yes

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, s) {
  const k1 = input.key; // SEED
  if (a[k1]) {
    a[k1][k2] = s[k1][k2]; // SINK
  }
};
```

## skx-if-property-no

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, v) {
  const k1 = input.key; // SEED
  if (a[k1]) {
    a[k1][k2] = v; // SINK
  }
};
```

## skx-compound-function-yes

expect: fires

```js
function put(a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1][k2] += s[k1][k2]; // SINK
}
```

## skx-compound-function-no

expect: fires

```js
function put(a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1][k2] += v; // SINK
}
```

## skx-compound-method-yes

expect: fires

```js
const writer = {
  put(a, input, k2, s) {
    const k1 = input.key; // SEED
    a[k1][k2] += s[k1][k2]; // SINK
  }
};
```

## skx-compound-method-no

expect: fires

```js
const writer = {
  put(a, input, k2, v) {
    const k1 = input.key; // SEED
    a[k1][k2] += v; // SINK
  }
};
```

## skx-compound-arrow-yes

expect: fires

```js
const put = (a, input, k2, s) => {
  const k1 = input.key; // SEED
  a[k1][k2] += s[k1][k2]; // SINK
};
```

## skx-compound-arrow-no

expect: fires

```js
const put = (a, input, k2, v) => {
  const k1 = input.key; // SEED
  a[k1][k2] += v; // SINK
};
```

## skx-compound-property-yes

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1][k2] += s[k1][k2]; // SINK
};
```

## skx-compound-property-no

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1][k2] += v; // SINK
};
```

## skx-logical-function-yes

expect: fires

```js
function put(a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1][k2] ??= s[k1][k2]; // SINK
}
```

## skx-logical-function-no

expect: fires

```js
function put(a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1][k2] ??= v; // SINK
}
```

## skx-logical-method-yes

expect: fires

```js
const writer = {
  put(a, input, k2, s) {
    const k1 = input.key; // SEED
    a[k1][k2] ??= s[k1][k2]; // SINK
  }
};
```

## skx-logical-method-no

expect: fires

```js
const writer = {
  put(a, input, k2, v) {
    const k1 = input.key; // SEED
    a[k1][k2] ??= v; // SINK
  }
};
```

## skx-logical-arrow-yes

expect: fires

```js
const put = (a, input, k2, s) => {
  const k1 = input.key; // SEED
  a[k1][k2] ??= s[k1][k2]; // SINK
};
```

## skx-logical-arrow-no

expect: fires

```js
const put = (a, input, k2, v) => {
  const k1 = input.key; // SEED
  a[k1][k2] ??= v; // SINK
};
```

## skx-logical-property-yes

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1][k2] ??= s[k1][k2]; // SINK
};
```

## skx-logical-property-no

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1][k2] ??= v; // SINK
};
```

## skx-walk-function-yes

expect: fires

```js
function put(a, input, keys) {
  const k1 = input.key; // SEED
  for (const k2 of keys) {
    a = a[k1][k2] = a[k1][k2] || {}; // SINK
  }
}
```

## skx-walk-function-no

expect: fires

```js
function put(a, input, keys) {
  const k1 = input.key; // SEED
  for (const k2 of keys) {
    const next = a[k1][k2];
    a = a[k1][k2] = next || {}; // SINK
  }
}
```

## skx-walk-method-yes

expect: fires

```js
const writer = {
  put(a, input, keys) {
    const k1 = input.key; // SEED
    for (const k2 of keys) {
      a = a[k1][k2] = a[k1][k2] || {}; // SINK
    }
  }
};
```

## skx-walk-method-no

expect: fires

```js
const writer = {
  put(a, input, keys) {
    const k1 = input.key; // SEED
    for (const k2 of keys) {
      const next = a[k1][k2];
      a = a[k1][k2] = next || {}; // SINK
    }
  }
};
```

## skx-walk-arrow-yes

expect: fires

```js
const put = (a, input, keys) => {
  const k1 = input.key; // SEED
  for (const k2 of keys) {
    a = a[k1][k2] = a[k1][k2] || {}; // SINK
  }
};
```

## skx-walk-arrow-no

expect: fires

```js
const put = (a, input, keys) => {
  const k1 = input.key; // SEED
  for (const k2 of keys) {
    const next = a[k1][k2];
    a = a[k1][k2] = next || {}; // SINK
  }
};
```

## skx-walk-property-yes

expect: fires

```js
const writer = {};
writer.put = function (a, input, keys) {
  const k1 = input.key; // SEED
  for (const k2 of keys) {
    a = a[k1][k2] = a[k1][k2] || {}; // SINK
  }
};
```

## skx-walk-property-no

expect: fires

```js
const writer = {};
writer.put = function (a, input, keys) {
  const k1 = input.key; // SEED
  for (const k2 of keys) {
    const next = a[k1][k2];
    a = a[k1][k2] = next || {}; // SINK
  }
};
```

## skx-multiline-function-yes

expect: fires

```js
function put(a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1] // SINK
    [k2] = s[k1][k2];
}
```

## skx-multiline-function-no

expect: fires

```js
function put(a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1] // SINK
    [k2] = v;
}
```

## skx-multiline-method-yes

expect: fires

```js
const writer = {
  put(a, input, k2, s) {
    const k1 = input.key; // SEED
    a[k1] // SINK
      [k2] = s[k1][k2];
  }
};
```

## skx-multiline-method-no

expect: fires

```js
const writer = {
  put(a, input, k2, v) {
    const k1 = input.key; // SEED
    a[k1] // SINK
      [k2] = v;
  }
};
```

## skx-multiline-arrow-yes

expect: fires

```js
const put = (a, input, k2, s) => {
  const k1 = input.key; // SEED
  a[k1] // SINK
    [k2] = s[k1][k2];
};
```

## skx-multiline-arrow-no

expect: fires

```js
const put = (a, input, k2, v) => {
  const k1 = input.key; // SEED
  a[k1] // SINK
    [k2] = v;
};
```

## skx-multiline-property-yes

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1] // SINK
    [k2] = s[k1][k2];
};
```

## skx-multiline-property-no

expect: fires

```js
const writer = {};
writer.put = function (a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1] // SINK
    [k2] = v;
};
```

## skx-as-string-function-yes

expect: fires

```ts
function put(a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1 as string][k2] = s[k1 as string][k2]; // SINK
}
```

## skx-as-string-function-no

expect: fires

```ts
function put(a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1 as string][k2] = v; // SINK
}
```

## skx-as-string-method-yes

expect: fires

```ts
const writer = {
  put(a, input, k2, s) {
    const k1 = input.key; // SEED
    a[k1 as string][k2] = s[k1 as string][k2]; // SINK
  }
};
```

## skx-as-string-method-no

expect: fires

```ts
const writer = {
  put(a, input, k2, v) {
    const k1 = input.key; // SEED
    a[k1 as string][k2] = v; // SINK
  }
};
```

## skx-as-string-arrow-yes

expect: fires

```ts
const put = (a, input, k2, s) => {
  const k1 = input.key; // SEED
  a[k1 as string][k2] = s[k1 as string][k2]; // SINK
};
```

## skx-as-string-arrow-no

expect: fires

```ts
const put = (a, input, k2, v) => {
  const k1 = input.key; // SEED
  a[k1 as string][k2] = v; // SINK
};
```

## skx-as-string-property-yes

expect: fires

```ts
const writer = {};
writer.put = function (a, input, k2, s) {
  const k1 = input.key; // SEED
  a[k1 as string][k2] = s[k1 as string][k2]; // SINK
};
```

## skx-as-string-property-no

expect: fires

```ts
const writer = {};
writer.put = function (a, input, k2, v) {
  const k1 = input.key; // SEED
  a[k1 as string][k2] = v; // SINK
};
```

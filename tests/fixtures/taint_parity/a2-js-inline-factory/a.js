import express from 'express';
import { readdir as list } from 'node:fs';
const app = express();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
function lazy(factory) { return factory(); }
app.get('/view', lazy(() => function (req, res) {
  const value = req.query.value;
  list(value); // @SINK
}));

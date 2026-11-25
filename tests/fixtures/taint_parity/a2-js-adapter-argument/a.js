import express from 'express';
import { readdir as list } from 'node:fs';
const app = express();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
function keep(fn) { return fn; }
function view(req, res) {
  const value = req.query.value;
  list(value); // @SINK
}
app.get('/view', keep(view));

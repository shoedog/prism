import express from 'express';
import { readdir as list } from 'node:fs';
const app = express();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
function view(err, req, res, next) {
  const value = req.query.value;
  list(value); // @SINK
}
app.use(view);

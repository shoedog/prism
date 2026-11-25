import express from 'express';
import { exec } from 'child_process';
const app = express();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
function view(req, res) {
  const value = req.query.value;
  exec(value); // @SINK
}
app.get('/view', view);

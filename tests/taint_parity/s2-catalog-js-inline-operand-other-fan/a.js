import express from 'express';
const app = express();
app.get('/view', (req, res) => {
  const t = acquire(); // @SRC
  observe(t); return fetch(req.query.url); // @SINK
});

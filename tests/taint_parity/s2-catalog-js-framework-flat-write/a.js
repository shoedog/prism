import express from 'express';
const app = express();
app.get('/view', (req, res) => {
  const t = acquire(); // @SRC
  observe(req.query.term); node.outerHTML = t; // @SINK
});

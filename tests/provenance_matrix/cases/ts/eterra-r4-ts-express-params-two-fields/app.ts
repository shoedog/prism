import express from 'express';
const app = express();
app.get('/:id', (req, res) => {
  const id = req.params.id;
  const method = req.method;
  res.send([id, method]);
});

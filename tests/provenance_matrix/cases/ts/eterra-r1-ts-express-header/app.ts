import express from 'express';
const app = express();
app.get('/x', (req, res) => {
  const token = req.header('x-token');
  res.send(token);
});

import express from 'express';
const app = express();
app.get('/', (req, res) => {
  req = { query: { q: 'fixed' } } as any;
  const value = req.query.q;
  res.send(value);
});

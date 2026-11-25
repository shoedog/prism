import express from 'express';
const app = express();
app.get('/', (req, res) => {
  function inner(req: {query: {q: string}}) {
    const term = req.query.q;
    return term;
  }
  res.send(inner({query: {q: 'fixed'}}));
});

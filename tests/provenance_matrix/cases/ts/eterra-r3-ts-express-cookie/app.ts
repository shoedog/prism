import express from 'express';
const app = express();
app.get('/', (req, res) => {
  const session = req.cookies.session;
  res.send(session);
});

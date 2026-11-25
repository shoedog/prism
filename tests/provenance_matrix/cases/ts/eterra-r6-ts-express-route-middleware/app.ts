import express from 'express';
const app = express();
app.post('/submit', express.json(), (req, res) => {
  const title = req.body.title;
  res.send(title);
});

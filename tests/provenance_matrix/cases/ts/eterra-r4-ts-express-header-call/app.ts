import express from 'express';
const app = express();
app.get('/', (req, res) => {
  const agent = req.get('User-Agent');
  res.send(agent);
});

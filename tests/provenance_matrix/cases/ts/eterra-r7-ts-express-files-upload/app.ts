import express from 'express';
const app = express();
app.post('/upload', (req, res) => {
  const upload = req.files;
  return res.send(upload);
});

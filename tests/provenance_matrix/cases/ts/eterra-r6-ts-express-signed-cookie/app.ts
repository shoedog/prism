import express from 'express';
const app = express();
app.get('/account', (req, res) => {
  const session = req.signedCookies.session;
  res.send(session);
});

import express from 'express';
const app = express();
app.use((req, res, next) => {
  next();
});

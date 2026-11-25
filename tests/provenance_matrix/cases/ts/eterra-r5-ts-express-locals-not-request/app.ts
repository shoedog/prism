import express from "express";
const app = express();
app.get("/", (req, res) => {
  const value = res.locals.name;
  res.send(value);
});

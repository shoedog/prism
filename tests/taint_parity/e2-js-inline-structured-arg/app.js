import express from "express";
const app = express();
app.get("/", (req, res) => { // @SRC
  return fetch(req.query.url); // @SINK
});

import express from "express";
const app = express();
app.get("/", (req, res) => {
  const request = req; // @SRC
  observe(request.query.term); return res.send(request.query.term); // @SINK
});

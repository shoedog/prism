import express from "express";
import path from "path";

const app = express();

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  return res.sendFile(path.join("/uploads", filename));
});


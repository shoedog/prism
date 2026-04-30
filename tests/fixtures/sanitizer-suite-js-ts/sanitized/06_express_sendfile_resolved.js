import express from "express";
import path from "path";

const app = express();
const uploadsDir = "/uploads/";

app.get("/download/:name", (req, res) => {
  const filename = req.params.name;
  const resolved = path.resolve(uploadsDir, filename);
  if (!resolved.startsWith(uploadsDir)) {
    return;
  }
  return res.sendFile(resolved);
});

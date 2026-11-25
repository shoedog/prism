import express, { Request, Response } from "express";
const app = express();
app.get("/users/:id", (req: Request, res: Response) => {
  const id = req.params.id;
  res.send(id);
});

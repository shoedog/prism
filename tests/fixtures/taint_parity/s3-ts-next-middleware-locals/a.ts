import { exec } from 'child_process';
type Response = { locals: { payload?: string } };
function emit(res: Response) {
  const x = res.locals.payload!;
  exec(x); // @SINK
}
function route(res: Response, next: () => void) {
  const t = acquire(); // @SRC
  res.locals.payload = t;
  return next();
}
function entry() {
  const res: Response = { locals: {} };
  route(res, () => emit(res));
}
